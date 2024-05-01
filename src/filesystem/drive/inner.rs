use std::collections::{HashMap, HashSet};

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use slab::Slab;
use tracing::instrument;
use winnow::binary::le_u64;
use winnow::Parser;

use crate::codec::crypto::AccessKey;
use crate::codec::*;
use crate::filesystem::drive::DriveAccess;
use crate::filesystem::nodes::{Node, NodeBuilder, NodeId};
use crate::prelude::nodes::NodeData;
use crate::utils::std_io_err;

use super::OperationError;

pub(crate) struct InnerDrive {
    access: DriveAccess,

    journal_start: JournalCheckpoint,
    root_pid: PermanentId,

    nodes: Slab<Node>,
    permanent_id_map: HashMap<PermanentId, NodeId>,

    dirty_nodes: Vec<NodeId>,
}

impl InnerDrive {
    /// Returns an immutable reference to the [`DriveAccess`] of this [`InnerDrive`]
    pub(crate) fn access(&self) -> &DriveAccess {
        &self.access
    }

    /// Returns an mutable reference to the [`DriveAccess`] of this [`InnerDrive`]
    pub(crate) fn access_mut(&mut self) -> &mut DriveAccess {
        &mut self.access
    }

    /// Returns an immutable reference to the contained [`Node`] with the passed in [`NodeId`]
    /// # Error
    /// - [`OperationError::InternalCorruption`] if the [`NodeId`] is not found
    pub(crate) fn by_id(&self, node_id: NodeId) -> Result<&Node, OperationError> {
        self.nodes
            .get(node_id)
            .ok_or(OperationError::InternalCorruption(
                node_id,
                "missing expected node ID",
            ))
    }

    /// Returns an mutable reference to the contained [`Node`] with the passed in [`NodeId`]
    /// # Error
    /// - [`OperationError::InternalCorruption`] if the [`NodeId`] is not found
    pub(crate) async fn by_id_mut(&mut self, node_id: NodeId) -> Result<&mut Node, OperationError> {
        self.mark_ancestors_dirty(node_id).await?;
        self.nodes
            .get_mut(node_id)
            .ok_or(OperationError::InternalCorruption(
                node_id,
                "missing expected node ID",
            ))
    }

    async fn mark_ancestors_dirty(&mut self, node_id: NodeId) -> Result<(), OperationError> {
        // Changes have happened in `node_id` walk up its parents to root marking nodes as dirty
        let mut node_id = node_id;
        self.dirty_nodes.push(node_id);

        let _node_mut = self //mark cid dirty (getting mutable access to its data will cause this)
            .nodes
            .get_mut(node_id)
            .expect("This succeeded directly above in non-mut form")
            .data_mut()
            .await;
        while let Some(parent_perm_id) = self.by_id(node_id)?.parent_id() {
            node_id = self.lookup_internal_id(&parent_perm_id)?;
            self.dirty_nodes.push(node_id);
        }
        let _node_mut = self //mark cid dirty (getting mutable access to its data will cause this)
            .nodes
            .get_mut(node_id)
            .expect("This succeeded directly above in non-mut form")
            .data_mut()
            .await;
        Ok(())
    }

    pub(crate) async fn clean_drive(&mut self) -> Result<(), OperationError> {
        // Take the dirty node list (replacing it with an empty Vec)
        let mut dirty = std::mem::take(&mut self.dirty_nodes);

        // Popping from back of the dirty list, push NodeIds into `node_list`
        // Only pushing if the list does not already contain that nodeId.
        let mut node_list = Vec::new();
        while let Some(node_id) = dirty.pop() {
            if !node_list.contains(&node_id) {
                node_list.push(node_id);
            }
        }

        // Pop elements from back and update their size and Cid
        while let Some(node_id) = node_list.pop() {
            // Because of the work above we can assume that once we get here all of a nodes children are up to date
            let node = self.by_id(node_id)?;

            // Update Size:
            let new_children_size = node.ordered_child_pids().iter().fold(0, |acc, child_pid| {
                // This should maybe actually float up an error as opposed to defaulting to 0 if a child can not be found
                // It implies a child on the current node can't be found in the filesystem.
                // This should be done carefully though, maybe finishing the res of its work before returning an error? Or maybe not?
                let child_size = self.by_perm_id(child_pid).ok().map_or(0, Node::size);
                acc + child_size
            });
            let node_mut = self.by_id_mut(node_id).await.unwrap();
            match node_mut.data_mut().await {
                NodeData::Directory { children_size, .. } => *children_size = new_children_size,
                _ => {}
            }
        }
        Ok(())
    }

    /// Returns an immutable reference to the contained [`Node`] with the passed in [`PermanentId`]
    /// # Error
    /// - [`OperationError::MissingPermanentId`] if the [`PermanentId`] is not found
    /// - [`OperationError::InternalCorruption`] if the [`PermanentId`] maps to a [`NodeId`] that no longer exists
    pub(crate) fn by_perm_id(&self, permanent_id: &PermanentId) -> Result<&Node, OperationError> {
        let node_id = self.lookup_internal_id(permanent_id)?;
        self.by_id(node_id)
    }

    /// Returns an mutable reference to the contained [`Node`] with the passed in [`PermanentId`]
    /// # Error
    /// - [`OperationError::MissingPermanentId`] if the [`PermanentId`] is not found
    /// - [`OperationError::InternalCorruption`] if the [`PermanentId`] maps to a [`NodeId`] that no longer exists
    pub(crate) async fn by_perm_id_mut(
        &mut self,
        permanent_id: &PermanentId,
    ) -> Result<&mut Node, OperationError> {
        let node_id = self.lookup_internal_id(permanent_id)?;
        self.by_id_mut(node_id).await
    }

    /// Creates a new [`Node`] using the passed in builder function
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

        let parent_node = self.by_perm_id_mut(&parent_permanent_id).await?;
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

        // `Vec::pop` pops from the back of the Vec so we will implicitly be getting the correct ordering as
        // we pop elements in the reverse order we found them in our DFS

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
            dirty_nodes: Vec::new(),
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

    /// Returns an iterator of immutable references to every [`Node`] in this [`InnerDrive`]
    pub(crate) fn node_iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().map(|(_, node)| node)
    }

    pub(crate) fn parse<'a>(
        input: Stream<'a>,
        drive_access: DriveAccess,
        journal_start: JournalCheckpoint,
        data_key: Option<&AccessKey>,
    ) -> ParserResult<'a, Self> {
        tracing::trace!(available_data = ?input.len(), "inner_drive::parse");

        let (remaining, root_pid) = PermanentId::parse(input)?;
        let bytes_read = input.len() - remaining.len();

        let (remaining, node_count) = le_u64.parse_peek(remaining)?;
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

                    // Error is disabled in builds without the `strict` feature
                    // for now since there are some existing filesytems out there that
                    // have the "backwards" encoding order

                    #[cfg(feature = "strict")]
                    return Err(winnow::error::ErrMode::Cut(
                        winnow::error::ParserError::from_error_kind(
                            &node_input,
                            winnow::error::ErrorKind::Verify,
                        ),
                    ));
                }
            }

            permanent_id_map.insert(permanent_id, node_id);

            entry.insert(node);
        }

        let root_node_id = *permanent_id_map.get(&root_pid).ok_or_else(|| {
            winnow::error::ErrMode::Cut(winnow::error::ParserError::from_error_kind(
                &node_input,
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
            dirty_nodes: Vec::new(),
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

        let parent_node = self.by_perm_id_mut(&parent_perm_id).await?;
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

#[cfg(test)]
pub(crate) mod test {
    use crate::filesystem::nodes::NodeName;
    use crate::prelude::*;

    use winnow::Partial;

    use super::*;

    fn initialize_inner_drive() -> (ActorId, InnerDrive) {
        let mut rng = crate::utils::crypto_rng();

        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        let actor_id = verifying_key.actor_id();

        let access = DriveAccess::initialize(&mut rng, verifying_key).unwrap();

        (
            actor_id,
            InnerDrive::initialize(&mut rng, actor_id, access).unwrap(),
        )
    }

    #[test]
    fn test_drive_initialization() {
        let (_, inner) = initialize_inner_drive();
        assert!(inner.nodes.capacity() == 32);
        assert!(inner.nodes.len() == 1);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_node_creation() {
        let mut rng = crate::utils::crypto_rng();
        let (actor_id, mut inner) = initialize_inner_drive();

        let create_node_res = inner
            .create_node(
                &mut rng,
                actor_id,
                inner.root_pid(),
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::directory(NodeName::try_from("test").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await;
        let node_pid = create_node_res.unwrap();
        let new_node = inner.by_perm_id(&node_pid).unwrap();
        assert_eq!(new_node.name(), NodeName::try_from("test").unwrap());
        assert_eq!(new_node.parent_id().unwrap(), inner.root_pid);
        let root_children = inner.root_node().unwrap().ordered_child_pids();
        assert_eq!(root_children.len(), 1);
        assert_eq!(root_children.get(0).unwrap(), &node_pid);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_drive_round_tripping() {
        let inner = build_interesting_inner().await;

        let access = inner.access();
        let journal = inner.journal_start();
        let mut encoded = Vec::new();

        let encoding_res = inner.encode(&mut encoded).await;
        assert!(encoding_res.is_ok());

        let (remaining, parsed) = InnerDrive::parse(
            Partial::new(encoded.as_slice()),
            access.to_owned(),
            journal,
            None,
        )
        .unwrap();
        assert!(remaining.is_empty());
        assert_eq!(inner.nodes.len(), parsed.nodes.len());
        for (_, node) in inner.nodes {
            let pid = node.permanent_id();
            assert!(parsed.lookup_internal_id(&pid).is_ok())
        }
    }

    // A fixture to make a relatively interesting inner
    pub(crate) async fn build_interesting_inner() -> InnerDrive {
        let mut rng = crate::utils::crypto_rng();
        let (actor_id, mut inner) = initialize_inner_drive();

        //           -----file_1
        //         /
        // root ---------file_2
        //         \
        //          --------- dir_1 ----- dir_2 ---- dir_3 ---- file_3
        //                         \
        //                           --- file_4
        //                            \
        //                              ----file_5

        let _file_1 = inner
            .create_node(
                &mut rng,
                actor_id,
                inner.root_pid(),
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::file(NodeName::try_from("file_1").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await
            .unwrap();
        let _file_2 = inner
            .create_node(
                &mut rng,
                actor_id,
                inner.root_pid(),
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::file(NodeName::try_from("file_2").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await
            .unwrap();
        let dir_1 = inner
            .create_node(
                &mut rng,
                actor_id,
                inner.root_pid(),
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::directory(NodeName::try_from("dir_1").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await
            .unwrap();

        let dir_2 = inner
            .create_node(
                &mut rng,
                actor_id,
                dir_1,
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::directory(NodeName::try_from("dir_2").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await
            .unwrap();

        let dir_3 = inner
            .create_node(
                &mut rng,
                actor_id,
                dir_2,
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::directory(NodeName::try_from("dir_3").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await
            .unwrap();

        let _file_4 = inner
            .create_node(
                &mut rng,
                actor_id,
                dir_2,
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::directory(NodeName::try_from("file_4").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await
            .unwrap();

        let _file_5 = inner
            .create_node(
                &mut rng,
                actor_id,
                dir_2,
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::directory(NodeName::try_from("file_5").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await
            .unwrap();

        let _file_3 = inner
            .create_node(
                &mut rng,
                actor_id,
                dir_3,
                |rng, new_node_id, parent_id, actor_id| async move {
                    NodeBuilder::directory(NodeName::try_from("file_3").unwrap())
                        .with_parent(parent_id)
                        .with_id(new_node_id)
                        .with_owner(actor_id)
                        .build(rng)
                        .map_err(OperationError::CreationFailed)
                },
            )
            .await
            .unwrap();

        inner
    }
}
