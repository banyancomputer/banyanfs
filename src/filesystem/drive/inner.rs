use std::collections::{HashMap, HashSet};
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u64;
use slab::Slab;
use tracing::instrument;

use crate::codec::crypto::AccessKey;
use crate::codec::*;
use crate::filesystem::drive::DriveAccess;
use crate::filesystem::nodes::{Node, NodeBuilder, NodeId};

use super::OperationError;

pub(crate) struct InnerDrive {
    access: DriveAccess,

    journal_start: JournalCheckpoint,
    root_node_id: NodeId,

    nodes: Slab<Node>,
    permanent_id_map: HashMap<PermanentId, NodeId>,
}

impl InnerDrive {
    pub(crate) fn access(&self) -> &DriveAccess {
        &self.access
    }

    pub(crate) fn by_id(&self, node_id: NodeId) -> Result<&Node, OperationError> {
        self.nodes
            .get(node_id)
            .ok_or(OperationError::InternalCorruption(
                node_id,
                "missing expected node ID",
            ))
    }

    pub(crate) fn by_id_mut(&mut self, node_id: NodeId) -> Result<&mut Node, OperationError> {
        self.nodes
            .get_mut(node_id)
            .ok_or(OperationError::InternalCorruption(
                node_id,
                "missing expected node ID",
            ))
    }

    pub(crate) fn by_perm_id(&self, permanent_id: &PermanentId) -> Result<&Node, OperationError> {
        let node_id = self
            .permanent_id_map
            .get(permanent_id)
            .ok_or(OperationError::MissingPermanentId(*permanent_id))?;

        self.by_id(*node_id)
    }

    pub(crate) fn by_perm_id_mut(
        &mut self,
        permanent_id: &PermanentId,
    ) -> Result<&mut Node, OperationError> {
        let node_id = self
            .permanent_id_map
            .get(permanent_id)
            .ok_or(OperationError::MissingPermanentId(*permanent_id))?;

        self.by_id_mut(*node_id)
    }

    #[instrument(level = tracing::Level::TRACE, skip(self, rng, build_node))]
    pub(crate) async fn create_node<'a, R, F, Fut>(
        &mut self,
        rng: &'a mut R,
        owner_id: ActorId,
        parent_permanent_id: PermanentId,
        build_node: F,
    ) -> Result<PermanentId, OperationError>
    where
        R: CryptoRngCore,
        F: FnOnce(&'a mut R, NodeId, PermanentId, ActorId) -> Fut,
        Fut: std::future::Future<Output = Result<Node, OperationError>>,
    {
        let parent_node = self.by_perm_id(&parent_permanent_id)?;
        if !parent_node.supports_children() {
            return Err(OperationError::ParentMustBeDirectory);
        }

        let node_entry = self.nodes.vacant_entry();
        let node_id = node_entry.key();

        let node = build_node(rng, node_id, parent_permanent_id, owner_id).await?;

        let name = node.name();
        let permanent_id = node.permanent_id();

        node_entry.insert(node);

        self.permanent_id_map.insert(permanent_id, node_id);

        let parent_node = self.by_perm_id_mut(&parent_permanent_id)?;
        parent_node.add_child(name, permanent_id).await?;

        Ok(permanent_id)
    }

    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        // We want to walk the nodes in a consistent depth first order to provide a total ordering
        // of the internal nodes. This is important as once we get to encoding we want to ensure
        // that any node that has children is encoded after its children. We accomplish this by
        // reversing the list from our walked tree and encoding in that order.
        //
        // This will silently discard any disconnected leaf nodes. Loops are tolerated.
        let mut ordered_ids = Vec::new();
        let mut seen_ids = HashSet::new();

        let root_perm_id = self
            .root_node()
            .map_err(|_| std_err("no root node"))?
            .permanent_id();

        let mut outstanding_ids = vec![root_perm_id];

        while let Some(node_pid) = outstanding_ids.pop() {
            let node = self
                .by_perm_id(&node_pid)
                .map_err(|_| std_err("missing node PID"))?;

            if seen_ids.contains(&node.permanent_id()) {
                continue;
            }

            seen_ids.insert(node.permanent_id());
            ordered_ids.push(node.permanent_id());
            outstanding_ids.extend(node.ordered_child_pids());
        }

        // Flip our ordering to encode our leaf nodes first. This allows us to include CIDs from
        // the leafs as part of the data encoding for any containing nodes.
        ordered_ids.reverse();

        let mut referenced_data_cids = HashSet::new();

        let mut node_buffer = Vec::new();
        while let Some(node_pid) = ordered_ids.pop() {
            let node = self
                .by_perm_id(&node_pid)
                .map_err(|_| std_err("missing node PID"))?;

            node.encode(&mut node_buffer).await?;

            // todo(sstelfox): data cids don't need to be globally ordered, lets lexigraphically
            // sort them.
            for cid in node.ordered_data_cids() {
                referenced_data_cids.insert(cid);
            }
        }

        // todo(sstelfox): should scan the slab for any nodes that are not reachable from the root and track
        // them for removal in the journal and maintenance logs. It really shoudn't happen but be
        // defensive against errors...

        let root_node = &self.nodes[self.root_node_id];
        let root_perm_id = root_node.permanent_id();
        let encoded_len = root_perm_id.encode(writer).await?;
        written_bytes += encoded_len;
        tracing::trace!(?root_perm_id, encoded_len, "node_encoding::root_perm_id");

        let node_count = seen_ids.len() as u64;
        let node_count_bytes = node_count.to_le_bytes();
        writer.write_all(&node_count_bytes).await?;
        written_bytes += node_count_bytes.len();
        tracing::trace!(
            ?node_count,
            encode_len = node_count_bytes.len(),
            "node_encoding::node_count"
        );

        writer.write_all(&node_buffer).await?;
        written_bytes += node_buffer.len();

        Ok(written_bytes)
    }

    pub(crate) fn initialize(
        rng: &mut impl CryptoRngCore,
        actor_id: ActorId,
        access: DriveAccess,
    ) -> Result<Self, OperationError> {
        let journal_start = JournalCheckpoint::initialize();

        let mut nodes = Slab::with_capacity(32);
        let mut permanent_id_map = HashMap::new();

        let node_entry = nodes.vacant_entry();
        let root_node_id = node_entry.key();

        let directory = NodeBuilder::root()
            .with_id(root_node_id)
            .with_owner(actor_id)
            .build(rng)?;

        permanent_id_map.insert(directory.permanent_id(), root_node_id);
        node_entry.insert(directory);

        let inner = Self {
            access,
            journal_start,

            nodes,
            root_node_id,
            permanent_id_map,
        };

        Ok(inner)
    }

    pub(crate) fn journal_start(&self) -> JournalCheckpoint {
        self.journal_start.clone()
    }

    pub(crate) fn parse<'a>(
        input: &'a [u8],
        drive_access: DriveAccess,
        journal_start: JournalCheckpoint,
        data_key: Option<&AccessKey>,
    ) -> ParserResult<'a, Self> {
        tracing::trace!(available_data = ?input.len(), "inner_drive::parse");

        let (remaining, root_perm_id) = PermanentId::parse(input)?;
        let bytes_read = input.len() - remaining.len();
        tracing::trace!(
            ?root_perm_id,
            bytes_read,
            remaining_len = ?remaining.len(),
            "inner_drive::parse::root_perm_id"
        );

        let (remaining, node_count) = le_u64(remaining)?;
        let bytes_read = input.len() - remaining.len() - bytes_read;
        tracing::trace!(node_count, bytes_read,
            remaining_len = ?remaining.len(),
            "inner_drive::parse::node_count");

        let mut nodes = Slab::new();
        let mut permanent_id_map = HashMap::new();
        let mut expected_permanent_ids = HashSet::from([root_perm_id]);

        let mut node_input = remaining;
        for _ in 0..node_count {
            tracing::trace!(available_node_data = ?node_input.len(), "inner_drive::parse::node_loop");

            let entry = nodes.vacant_entry();
            let node_id = entry.key();

            let (remaining, (node, desired_node_ids)) = Node::parse(node_input, node_id, data_key)?;
            tracing::trace!(
                remaining_node_data = remaining.len(),
                desired_node_len = desired_node_ids.len(),
                "inner_drive::parse::node_loop::node"
            );
            node_input = remaining;

            for perm_id in desired_node_ids.into_iter() {
                expected_permanent_ids.insert(perm_id);
            }

            let permanent_id = node.permanent_id();
            if !expected_permanent_ids.contains(&permanent_id) {
                tracing::warn!(?permanent_id, ?node_id, node_kind = ?node.kind(), "found unexpected permanent ID in node data, skipping...");
                continue;
            }

            expected_permanent_ids.remove(&permanent_id);
            tracing::trace!(?permanent_id, ?node_id, node_kind = ?node.kind(), "inner_drive::parse::node");

            entry.insert(node);

            permanent_id_map.insert(permanent_id, node_id);
        }

        if !expected_permanent_ids.is_empty() {
            tracing::warn!(
                ?expected_permanent_ids,
                "missing expected permanent IDs in node data, fs missing data..."
            );
        }

        let root_node_id = *permanent_id_map.get(&root_perm_id).ok_or_else(|| {
            nom::Err::Failure(nom::error::make_error(
                node_input,
                nom::error::ErrorKind::Verify,
            ))
        })?;

        tracing::trace!(?root_node_id, "inner_drive::parse::complete");

        let inner_drive = InnerDrive {
            access: drive_access,
            journal_start,
            root_node_id,
            nodes,
            permanent_id_map,
        };

        Ok((node_input, inner_drive))
    }

    pub(crate) async fn remove_node(&mut self, perm_id: PermanentId) -> Result<(), OperationError> {
        // We need to first make this node an orphan by removing it from its parent and marking the
        // parent as dirty.
        let target_node = self.by_perm_id(&perm_id)?;
        let parent_perm_id = target_node
            .parent_id()
            .ok_or(OperationError::OrphanNode(perm_id))?;

        let parent_node = self.by_perm_id_mut(&parent_perm_id)?;
        parent_node.remove_permanent_id(&perm_id).await?;

        let mut nodes_to_remove = vec![perm_id];
        let mut data_cids_removed = Vec::new();

        while let Some(next_node_perm_id) = nodes_to_remove.pop() {
            let node_id = self
                .permanent_id_map
                .remove(&next_node_perm_id)
                .ok_or(OperationError::MissingPermanentId(next_node_perm_id))?;

            let node = self
                .nodes
                .try_remove(node_id)
                .ok_or(OperationError::InternalCorruption(
                    node_id,
                    "missing node for removal",
                ))?;

            data_cids_removed.extend(node.data().ordered_data_cids());
            nodes_to_remove.extend(node.data().ordered_child_pids());
        }

        Ok(())
    }

    pub(crate) fn root_node(&self) -> Result<&Node, OperationError> {
        self.by_id(self.root_node_id)
    }

    pub(crate) fn root_node_id(&self) -> NodeId {
        self.root_node_id
    }
}

fn std_err(msg: &'static str) -> StdError {
    StdError::new(StdErrorKind::Other, msg)
}
