use std::collections::{HashMap, HashSet};

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use winnow::number::streaming::le_u64;
use slab::Slab;
use tracing::instrument;

use crate::codec::crypto::AccessKey;
use crate::codec::*;
use crate::filesystem::drive::DriveAccess;
use crate::filesystem::nodes::{Node, NodeBuilder, NodeId};
use crate::utils::std_io_err;

use super::OperationError;

pub(crate) struct InnerDrive {
    access: DriveAccess,

    journal_start: JournalCheckpoint,
    root_pid: PermanentId,

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
        let node_id = self.lookup_internal_id(permanent_id)?;
        self.by_id(node_id)
    }

    pub(crate) fn by_perm_id_mut(
        &mut self,
        permanent_id: &PermanentId,
    ) -> Result<&mut Node, OperationError> {
        let node_id = self.lookup_internal_id(permanent_id)?;
        self.by_id_mut(node_id)
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
        // This will silently discard any disconnected leaf nodes. Loops are tolerated by
        // deduplication of unique identifiers.
        let mut ordered_ids = Vec::new();
        let mut seen_ids = HashSet::new();
        let mut outstanding_ids = vec![self.root_pid];

        while let Some(node_pid) = outstanding_ids.pop() {
            let node = self
                .by_perm_id(&node_pid)
                .map_err(|_| std_io_err("missing node PID"))?;

            // todo(sstelfox): The encoding order here still isn't correct, but I've loosened the
            // strictness on the decoder to finish the last production pieces.
            let permanent_id = node.permanent_id();
            if seen_ids.contains(&permanent_id) {
                // We've already seen this ID, but it now needs to appear earlier in our encoding
                // so we have to append to the end of our list.
                let existing_pos = ordered_ids
                    .iter()
                    .position(|&pid| pid == permanent_id)
                    .ok_or(std_io_err("expected PID to already be present"))?;

                ordered_ids.remove(existing_pos);
                ordered_ids.push(permanent_id);

                // We don't need to do anything else for nodes we've already seen
                continue;
            }

            seen_ids.insert(permanent_id);
            ordered_ids.push(permanent_id);

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
                .map_err(|_| std_io_err("missing node PID"))?;

            node.encode(&mut node_buffer).await?;

            if let Some(data_cids) = node.data_cids() {
                for cid in data_cids {
                    referenced_data_cids.insert(cid);
                }
            }
        }

        // todo(sstelfox): should scan the slab for any nodes that are not reachable from the root and track
        // them for removal in the journal and maintenance logs. It really shoudn't happen but be
        // defensive against errors... It can wait until the journal is complete though.

        // Data cids don't need to be globally ordered, but should be consistent
        // sort we need to get them into a vec then sort them (lexographically)
        let mut data_cids = referenced_data_cids.into_iter().collect::<Vec<_>>();
        data_cids.sort();

        // todo(sstelfox): need to track removed nodes in the inner drive so we can report those in the
        // maintenance logs

        let encoded_len = self.root_pid.encode(writer).await?;
        written_bytes += encoded_len;

        let node_count = seen_ids.len() as u64;
        let node_count_bytes = node_count.to_le_bytes();
        writer.write_all(&node_count_bytes).await?;
        written_bytes += node_count_bytes.len();

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

        let root_node = NodeBuilder::root()
            .with_id(root_node_id)
            .with_owner(actor_id)
            .build(rng)?;

        let root_pid = root_node.permanent_id();
        permanent_id_map.insert(root_pid, root_node_id);
        node_entry.insert(root_node);

        let inner = Self {
            access,
            journal_start,

            nodes,
            root_pid,
            permanent_id_map,
        };

        Ok(inner)
    }

    pub(crate) fn journal_start(&self) -> JournalCheckpoint {
        self.journal_start.clone()
    }

    pub(crate) fn lookup_internal_id(
        &self,
        perm_id: &PermanentId,
    ) -> Result<NodeId, OperationError> {
        self.permanent_id_map
            .get(perm_id)
            .copied()
            .ok_or(OperationError::MissingPermanentId(*perm_id))
    }

    pub(crate) fn node_iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().map(|(_, node)| node)
    }

    pub(crate) fn parse<'a>(
        input: &'a [u8],
        drive_access: DriveAccess,
        journal_start: JournalCheckpoint,
        data_key: Option<&AccessKey>,
    ) -> ParserResult<'a, Self> {
        tracing::trace!(available_data = ?input.len(), "inner_drive::parse");

        let (remaining, root_pid) = PermanentId::parse(input)?;
        let bytes_read = input.len() - remaining.len();

        let (remaining, node_count) = le_u64(remaining)?;
        let bytes_read = input.len() - remaining.len() - bytes_read;
        tracing::trace!(node_count, bytes_read,
            remaining_len = ?remaining.len(),
            "inner_drive::parse::node_count");

        let mut nodes = Slab::new();
        let mut permanent_id_map = HashMap::new();

        let mut node_input = remaining;
        for _ in 0..node_count {
            let entry = nodes.vacant_entry();
            let node_id = entry.key();

            let (remaining, node) = Node::parse(node_input, node_id, data_key)?;
            node_input = remaining;
            let permanent_id = node.permanent_id();

            for pid in node.ordered_child_pids() {
                if !permanent_id_map.contains_key(&pid) {
                    tracing::warn!(?permanent_id, child_pid = ?pid, "encountered child PID before parent");

                    //return Err(winnow::error::ErrMode::Cut(winnow::error::make_error(
                    //    node_input,
                    //    winnow::error::ErrorKind::Verify,
                    //)));
                }
            }

            permanent_id_map.insert(permanent_id, node_id);

            entry.insert(node);
        }

        let root_node_id = *permanent_id_map.get(&root_pid).ok_or_else(|| {
            winnow::error::ErrMode::Cut(winnow::error::make_error(
                node_input,
                winnow::error::ErrorKind::Verify,
            ))
        })?;

        tracing::trace!(?root_node_id, "inner_drive::parse::complete");

        let inner_drive = InnerDrive {
            access: drive_access,
            journal_start,
            root_pid,
            nodes,
            permanent_id_map,
        };

        Ok((node_input, inner_drive))
    }

    pub(crate) async fn remove_node(
        &mut self,
        perm_id: PermanentId,
    ) -> Result<Option<Vec<Cid>>, OperationError> {
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

            if let Some(data_cids) = node.data_cids() {
                data_cids_removed.extend(data_cids);
            }

            nodes_to_remove.append(&mut node.data().ordered_child_pids());
        }

        if data_cids_removed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(data_cids_removed))
        }
    }

    pub(crate) fn root_node(&self) -> Result<&Node, OperationError> {
        self.by_perm_id(&self.root_pid)
    }

    pub(crate) fn root_pid(&self) -> PermanentId {
        self.root_pid
    }
}
