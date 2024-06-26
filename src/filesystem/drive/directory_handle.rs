use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use futures::future::BoxFuture;
use futures::FutureExt;
use tracing::{debug, instrument, trace, Instrument, Level};

use crate::codec::filesystem::NodeKind;
use crate::codec::*;

use crate::codec::crypto::{AccessKey, SigningKey};
use crate::codec::data_storage::{data_chunk::DataChunk, DataBlock};
use crate::codec::filesystem::BlockKind;
use crate::filesystem::drive::{DirectoryEntry, InnerDrive, OperationError, WalkState};
#[cfg(feature = "mime-type")]
use crate::filesystem::nodes::metadata::{MetadataKey, MimeGuesser};
use crate::filesystem::nodes::{Node, NodeData, NodeId, NodeName};
use crate::filesystem::{ContentLocation, ContentReference, FileContent, NodeBuilder};
use crate::stores::DataStore;

use self::filesystem::Permissions;

const MAX_PATH_DEPTH: usize = 32;

/// A handle on a specific directory, used to perform most operations on the filesystem itself.
/// Instances of these are safe to clone but each one will track its own current working directory.
/// Changing the directory of a clone for example does not update the original handle.
#[derive(Clone)]
pub struct DirectoryHandle {
    pub(crate) current_key: Arc<SigningKey>,
    pub(crate) cwd_id: NodeId,
    pub(crate) inner: Arc<RwLock<InnerDrive>>,
}

impl DirectoryHandle {
    /// Retrieves the [`DirectoryEntry`] for the current working directory.
    pub async fn entry(&self) -> Result<DirectoryEntry, OperationError> {
        let inner_read = self.inner.read().await;
        let node = inner_read.by_id(self.cwd_id)?;
        DirectoryEntry::try_from(node)
    }

    /// Allows traversing the filesystem both up and down. Does not allow invalid character in any
    /// of the path elements (primarily "/"). Will report an error if you attempt to traverse above
    /// the root of the filesystem or into/through an invalid node type.
    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn cd(&self, path: &[&str]) -> Result<DirectoryHandle, OperationError> {
        trace!(cwd_id = self.cwd_id, "directory::cd");

        let target_directory_id = if path.is_empty() {
            self.cwd_id
        } else {
            match walk_path(&self.inner, self.cwd_id, path, 0).await {
                Ok(WalkState::FoundNode { node_id }) => node_id,
                _ => return Err(OperationError::NotTraversable),
            }
        };

        let directory = DirectoryHandle {
            current_key: self.current_key.clone(),
            cwd_id: target_directory_id,
            inner: self.inner.clone(),
        };

        Ok(directory)
    }

    /// Changes the permission on the target node. Currently not implemented.
    pub async fn chmod(&self, _path: &[&str], _owner: Permissions) -> Result<(), OperationError> {
        unimplemented!()
    }

    /// Changes the owner of the target node. Currently not implemented
    pub async fn chown(&self, _path: &[&str], _owner: ActorId) -> Result<(), OperationError> {
        unimplemented!()
    }

    /// Retrieve the contents of a directory as a Vector of `DirectoryEntry`
    /// Passed in path is relative to the current working directory, if path is empty it will
    /// list contents of current working directory
    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn ls(&self, path: &[&str]) -> Result<Vec<DirectoryEntry>, OperationError> {
        trace!(cwd_id = self.cwd_id, "directory::ls");

        // These behaviors are slightly different mostly in the error cases, in the first case we
        // should be in a directory, any other state is an error. In the latter case, we can match
        // a specific node as well as a directory and should always succeed if we can reach the
        // node.
        let inner_read = self.inner.read().await;
        let children = if path.is_empty() {
            let current_node = inner_read.by_id(self.cwd_id)?;
            match current_node.data() {
                NodeData::Directory { children, .. } => children.values(),
                _ => {
                    return Err(OperationError::InternalCorruption(
                        self.cwd_id,
                        "current NodeId not a directory",
                    ))
                }
            }
        } else {
            let node_id = match walk_path(&self.inner, self.cwd_id, path, 0).await {
                Ok(WalkState::FoundNode { node_id }) => node_id,
                _ => return Err(OperationError::NotTraversable),
            };

            let listed_node = inner_read.by_id(node_id)?;

            match listed_node.data() {
                NodeData::Directory { children, .. } => children.values(),
                _ => {
                    let entry = DirectoryEntry::try_from(listed_node)?;
                    return Ok(vec![entry]);
                }
            }
        };

        let mut entries = Vec::new();

        for perm_id in children.into_iter().map(|entry| entry.permanent_id()) {
            let node = inner_read.by_perm_id(perm_id)?;
            let entry = DirectoryEntry::try_from(node)?;

            entries.push(entry);
        }

        trace!(?entries, "directory::ls::success");

        Ok(entries)
    }

    #[instrument(level = Level::TRACE, skip(current_key, inner))]
    pub(crate) async fn new(
        current_key: Arc<SigningKey>,
        cwd_id: NodeId,
        inner: Arc<RwLock<InnerDrive>>,
    ) -> Self {
        trace!("directory::new");

        Self {
            current_key,
            cwd_id,
            inner,
        }
    }

    #[instrument(level = tracing::Level::TRACE, skip_all)]
    async fn insert_node<'a, 'b, R, F, Fut>(
        &'b mut self,
        rng: &'a mut R,
        parent_permanent_id: PermanentId,
        build_node: F,
    ) -> Result<PermanentId, OperationError>
    where
        R: CryptoRngCore,
        F: FnOnce(&'a mut R, NodeId, PermanentId, ActorId) -> Fut,
        Fut: std::future::Future<Output = Result<Node, OperationError>>,
    {
        trace!("directory::insert_node");

        let mut inner_write = self.inner.write().in_current_span().await;

        let owner_id = self.current_key.actor_id();
        let new_permanent_id = inner_write
            .create_node(rng, owner_id, parent_permanent_id, build_node)
            .await?;
        inner_write.clean_drive().await?;
        Ok(new_permanent_id)
    }

    #[instrument(skip(self, rng))]
    pub async fn mkdir(
        &mut self,
        rng: &mut impl CryptoRngCore,
        path: &[&str],
        recursive: bool,
    ) -> Result<(), OperationError> {
        if path.is_empty() {
            return Err(OperationError::UnexpectedEmptyPath);
        }

        for _ in 0..MAX_PATH_DEPTH {
            match walk_path(&self.inner.clone(), self.cwd_id, path, 0).await? {
                WalkState::FoundNode { node_id } => {
                    debug!(node_id, "drive::mkdir::already_exists");

                    let inner_read = self.inner.read().await;
                    let node = inner_read.by_id(node_id)?;

                    match node.kind() {
                        NodeKind::Directory => return Ok(()),
                        NodeKind::File => return Err(OperationError::Exists(node_id)),
                        _ => unimplemented!(),
                    }
                }
                WalkState::MissingComponent {
                    working_directory_id,
                    missing_name,
                    remaining_path,
                } => {
                    trace!(cwd_id = working_directory_id, name = ?missing_name, "drive::mkdir::node_missing");

                    // When we're not recursing and there are more path components left, we have to
                    // abort early
                    if !recursive && !remaining_path.is_empty() {
                        trace!(?remaining_path, "drive::mkdir::not_recursive");
                        return Err(OperationError::PathNotFound);
                    }

                    let inner_read = self.inner.read().await;
                    let parent_permanent_id =
                        inner_read.by_id(working_directory_id)?.permanent_id();
                    drop(inner_read);

                    self.insert_node(
                        &mut *rng,
                        parent_permanent_id,
                        |rng, new_node_id, parent_id, actor_id| async move {
                            NodeBuilder::directory(missing_name)
                                .with_parent(parent_id)
                                .with_id(new_node_id)
                                .with_owner(actor_id)
                                .build(rng)
                                .map_err(OperationError::CreationFailed)
                        },
                    )
                    .await?;

                    if remaining_path.is_empty() {
                        trace!("drive::mkdir::complete");
                        return Ok(());
                    }
                }
            }
        }

        Err(OperationError::PathTooDeep)
    }

    #[instrument(level = Level::DEBUG, skip(self, _rng))]
    pub async fn mv(
        &mut self,
        _rng: &mut impl CryptoRngCore,
        src_path: &[&str],
        dst_path: &[&str],
    ) -> Result<(), OperationError> {
        // Get the NodeId of the Node we are moving
        let src_node_id = match walk_path(&self.inner, self.cwd_id, src_path, 0).await? {
            WalkState::FoundNode { node_id } => node_id,
            WalkState::MissingComponent { .. } => return Err(OperationError::PathNotFound),
        };

        // Get the NodeId of the target node's new parent after the move
        // as well as the new name of the target (if it is changing)
        let (dst_parent_id, new_dst_name) =
            match walk_path(&self.inner, self.cwd_id, dst_path, 0).await? {
                WalkState::FoundNode { node_id } => {
                    // Path to a directory was passed in as destination
                    // the source node will keep its current name
                    let inner_read = self.inner.read().await;
                    let found_node = inner_read.by_id(node_id)?;

                    // Make sure the target node is a directory
                    if found_node.kind() != NodeKind::Directory {
                        return Err(OperationError::ParentMustBeDirectory);
                    }

                    (node_id, inner_read.by_id(src_node_id)?.name())
                }
                WalkState::MissingComponent {
                    working_directory_id,
                    remaining_path,
                    ..
                } => {
                    // Destination was specified with a new name
                    //
                    // If remaining path is empty then we are moving into the current working directory
                    // (of this directory handle). If it is of length 1 then the last element is the
                    // name of the new parent
                    //
                    // If its length is > 1 then we would have to make directories
                    // which is not permitted
                    if remaining_path.len() > 1 {
                        return Err(OperationError::PathNotFound);
                    }
                    let inner_read = self.inner.read().await;
                    let found_node = inner_read.by_id(working_directory_id)?;

                    // Make sure the target node is a directory
                    if found_node.kind() != NodeKind::Directory {
                        return Err(OperationError::ParentMustBeDirectory);
                    }

                    (
                        working_directory_id,
                        NodeName::named(
                            dst_path
                                .last()
                                .ok_or(OperationError::UnexpectedEmptyPath)? //Maybe should use `expect` as getting here means we know dest had at least one entry
                                .to_string(),
                        )?,
                    )
                }
            };

        let mut inner_write = self.inner.write().await;
        let src_node_name = inner_write.by_id(src_node_id)?.name();
        let src_node_cid = inner_write.by_id(src_node_id)?.cid().await?;
        let src_node_size = inner_write.by_id(src_node_id)?.size();
        let src_node_perm_id = inner_write.by_id(src_node_id)?.permanent_id();
        let src_parent_perm_id = inner_write.by_id(src_node_id)?.parent_id().ok_or(
            OperationError::InternalCorruption(src_node_id, "src node has no parent"),
        )?;
        let src_parent_node = inner_write.by_perm_id_mut(&src_parent_perm_id).await?;

        // Remove target node from its current location by removing it as a child from its parent
        src_parent_node
            .remove_child(&src_node_name)
            .await
            .map_err(|_| {
                OperationError::InternalCorruption(
                    src_parent_node.id(),
                    "Could not remove target from parent",
                )
            })?;

        // A Failure from here on would leave the child orphaned or inconsistent, I don't think there is a
        // good way to make the move an atomic operation though...
        // The borrow checker won't let us get mutable references to all three nodes at once (src, dst, src_parent)
        // We could maybe extend `DriveInner` to do this operation internally to make it more atomic
        let dst_parent_node = inner_write.by_id_mut(dst_parent_id).await?;
        let dst_parent_node_perm_id = dst_parent_node.permanent_id();

        dst_parent_node
            .add_child(
                new_dst_name.clone(),
                src_node_perm_id,
                src_node_cid,
                src_node_size,
            )
            .await?;

        let src_node = inner_write.by_id_mut(src_node_id).await?;
        src_node.set_parent_id(dst_parent_node_perm_id).await;
        src_node.set_name(new_dst_name).await;

        inner_write.clean_drive().await?;

        Ok(())
    }

    #[instrument(level = Level::DEBUG, skip(self, store))]
    pub async fn rm(
        &mut self,
        store: &mut impl DataStore,
        path: &[&str],
    ) -> Result<(), OperationError> {
        if path.is_empty() {
            return Err(OperationError::UnexpectedEmptyPath);
        }

        let target_node_id = match walk_path(&self.inner, self.cwd_id, path, 0).await? {
            WalkState::FoundNode { node_id } => node_id,
            WalkState::MissingComponent { .. } => return Err(OperationError::PathNotFound),
        };

        let mut inner_write = self.inner.write().await;
        let target_node = inner_write.by_id(target_node_id)?;
        let target_perm_id = target_node.permanent_id();

        if let Some(removed_data_cids) = inner_write.remove_node(target_perm_id).await? {
            for cid in removed_data_cids.into_iter() {
                store.remove(cid, true).await?;
            }
        }

        inner_write.clean_drive().await?;
        Ok(())
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn size(&self) -> Result<u64, OperationError> {
        //let inner_read = self.inner.read().await;

        tracing::warn!("impl generic dir entry size / not yet implemented");

        Ok(0)
    }

    // todo(sstelfox): this really needs to return a stream which will be a breaking change to the
    // API. If anyone finds this know that's coming, though it shouldn't be that much of a change
    // on the consumer side.
    pub async fn read(
        &self,
        store: &impl DataStore,
        path: &[&str],
    ) -> Result<Vec<u8>, OperationError> {
        if path.is_empty() {
            return Err(OperationError::UnexpectedEmptyPath);
        }

        let inner_read = self.inner.read().await;
        let actor_id = self.current_key.actor_id();
        if !inner_read.access().has_read_access(&actor_id) {
            return Err(OperationError::AccessDenied);
        }
        drop(inner_read);

        let target_node_id = match walk_path(&self.inner, self.cwd_id, path, 0).await? {
            WalkState::FoundNode { node_id } => node_id,
            WalkState::MissingComponent { .. } => return Err(OperationError::PathNotFound),
        };

        let inner_read = self.inner.read().await;

        let read_node = inner_read.by_id(target_node_id)?;
        let node_content = match read_node.data() {
            NodeData::File { content, .. } => content,
            NodeData::AssociatedData { content, .. } => content,
            _ => return Err(OperationError::NotReadable),
        };

        if node_content.is_stub() {
            return Err(OperationError::NotAvailable);
        }

        if node_content.is_encrypted() {
            let locked_key = node_content
                .data_key()
                .map_err(|_| OperationError::AccessDenied)?;

            let data_key = match inner_read.access().data_key() {
                Some(data_key) => data_key,
                None => return Err(OperationError::AccessDenied),
            };

            let unlocked_key = locked_key
                .unlock(data_key)
                .map_err(|_| OperationError::AccessDenied)?;

            let mut file_data = Vec::new();

            for content_ref in node_content.content_references()? {
                if !store.contains_cid(content_ref.data_block_cid()).await? {
                    return Err(OperationError::BlockUnavailable(
                        content_ref.data_block_cid(),
                    ));
                }

                let data_chunk = store.retrieve(content_ref.data_block_cid()).await?;

                let (_remaining, block) = DataBlock::parse_with_magic(Stream::new(&data_chunk))
                    .map_err(|err| {
                        tracing::error!("parsing of data block failed: {err:?}");
                        OperationError::BlockCorrupted(content_ref.data_block_cid())
                    })?;
                // todo(sstelfox): still stuff remaining which means this decoder is sloppy
                //tracing::info!(?remaining, "drive::read::remaining");
                //debug_assert!(remaining.is_empty(), "no extra data should be present");

                for location in content_ref.chunks() {
                    if !matches!(location.block_kind(), BlockKind::Data) {
                        unimplemented!("indirect reference loading");
                    }

                    let encrypted_chunk = block
                        .get_chunk(location.block_index() as usize)
                        .map_err(|err| {
                            tracing::error!("failed to retrieve block chunk: {err:?}");
                            OperationError::BlockCorrupted(content_ref.data_block_cid())
                        })?;
                    let chunk = encrypted_chunk
                        .decrypt(&block.data_options(), &unlocked_key)
                        .map_err(|_| {
                            OperationError::BlockCorrupted(content_ref.data_block_cid())
                        })?;

                    // &unlocked_key,
                    // &verifying_key,

                    file_data.extend_from_slice(chunk.data());
                }
            }

            Ok(file_data)
        } else if node_content.is_empty() {
            Ok(Vec::new())
        } else {
            unimplemented!()
        }
    }

    #[instrument(level = Level::DEBUG, skip(self, rng, store))]
    pub async fn write(
        &mut self,
        rng: &mut impl CryptoRngCore,
        store: &mut impl DataStore,
        path: &[&str],
        data: &[u8],
    ) -> Result<(), OperationError> {
        if path.is_empty() {
            return Err(OperationError::UnexpectedEmptyPath);
        }

        let inner_read = self.inner.read().await;
        let actor_id = self.current_key.actor_id();
        if !inner_read.access().has_write_access(&actor_id) {
            return Err(OperationError::AccessDenied);
        }

        let data_key = match inner_read.access().data_key() {
            Some(data_key) => data_key.clone(),
            None => return Err(OperationError::AccessDenied),
        };

        drop(inner_read);

        let existing_file = match walk_path(&self.inner, self.cwd_id, path, 0).await {
            Ok(WalkState::FoundNode { node_id }) => Some(node_id),
            _ => None,
        };

        let data_size = data.len() as u64;
        let new_permanent_id = match existing_file {
            Some(existing_file) => {
                let inner_read = self.inner.read().await;
                let node = inner_read.by_id(existing_file)?;
                node.permanent_id()
            }
            None => {
                let (parent_path, name) = path.split_at(path.len() - 1);
                let file_name = NodeName::try_from(name[0]).map_err(OperationError::InvalidName)?;

                tracing::info!(?path, ?file_name, "drive::write");

                let parent_id = match walk_path(&self.inner, self.cwd_id, parent_path, 0).await? {
                    WalkState::FoundNode { node_id } => node_id,
                    WalkState::MissingComponent { .. } => return Err(OperationError::PathNotFound),
                };

                tracing::info!(?parent_id, ?parent_path, "drive::write::parent_id");

                let inner_read = self.inner.read().await;
                let parent_node = inner_read.by_id(parent_id)?;
                let parent_perm_id = parent_node.permanent_id();
                drop(inner_read);

                let node_name = file_name.clone();
                self.insert_node(
                    rng,
                    parent_perm_id,
                    |rng, new_node_id, parent_id, actor_id| async move {
                        NodeBuilder::file(node_name)
                            .with_parent(parent_id)
                            .with_id(new_node_id)
                            .with_owner(actor_id)
                            .with_size_hint(data_size)
                            .build(rng)
                            .map_err(OperationError::CreationFailed)
                    },
                )
                .await?
            }
        };

        if data.is_empty() {
            let mut inner_write = self.inner.write().await;
            let node = inner_write.by_perm_id_mut(&new_permanent_id).await?;
            let node_data = node.data_mut().await;
            *node_data = NodeData::empty_file();
            return Ok(());
        }

        let small_block_threshold: usize = DataBlock::small_encrypted_data_size() * 8;
        let block_creator = if data_size > small_block_threshold as u64 {
            || {
                DataBlock::small().map_err(|err| {
                    tracing::error!("failed to create data block: {:?}", err);
                    OperationError::Other("data block failed")
                })
            }
        } else {
            || {
                DataBlock::standard().map_err(|err| {
                    tracing::error!("failed to create data block: {:?}", err);
                    OperationError::Other("data block failed")
                })
            }
        };

        // todo(sstelfox): bit lazy here, should calculate this as I stream it but speed right
        // now...
        let plaintext_cid = crate::utils::calculate_cid(data);

        let mut remaining_data = data;
        let mut active_block = block_creator()?;
        let active_block_chunk_size = active_block.data_options().chunk_data_size();
        let node_data_key = AccessKey::generate(rng);
        let mut content_references = Vec::new();
        let mut content_indexes = Vec::new();

        while !remaining_data.is_empty() {
            let data_to_read = std::cmp::min(remaining_data.len(), active_block_chunk_size);
            let (chunk_data, next_data) = remaining_data.split_at(data_to_read);
            remaining_data = next_data;

            let chunk = DataChunk::from_slice(chunk_data, &active_block.data_options())
                .map_err(|err| {
                    tracing::error!("failed to push chunk: {:?}", err);
                    OperationError::Other("expected remaining capacity")
                })?
                .encrypt(rng, &active_block.data_options(), &node_data_key)
                .await
                .map_err(|err| {
                    tracing::error!("Failed to encrypt chunk: {:?}", err);
                    OperationError::Other("Error encrypting chunk")
                })?;

            content_indexes.push(active_block.push_chunk(chunk).map_err(|err| {
                tracing::error!("failed to push chunk: {:?}", err);
                OperationError::Other("expected remaining capacity")
            })?);

            if active_block.is_full() {
                let mut sealed_block = Vec::new();

                let (_, cids) =
                    active_block
                        .encode(rng, &mut sealed_block)
                        .await
                        .map_err(|err| {
                            tracing::error!("failed to encode block: {:?}", err);
                            OperationError::Other("failed to encode block")
                        })?;

                let cid = active_block
                    .cid()
                    .map_err(|_| OperationError::Other("unable to access block cid"))?;

                store.store(cid.clone(), sealed_block, false).await?;

                let locations = content_indexes
                    .iter()
                    .map(|i| ContentLocation::data(cids[*i].clone(), *i as u64))
                    .collect::<Vec<_>>();
                content_indexes.clear();

                let content_ref =
                    ContentReference::new(cid, active_block.data_options(), locations);
                content_references.push(content_ref);

                active_block = block_creator()?;
            }
        }

        if !active_block.is_empty() {
            tracing::info!("writing trailing block");

            // todo(sstelfox): this is duplicated, need to extract it
            let mut sealed_block = Vec::new();

            let (_, cids) = active_block
                .encode(rng, &mut sealed_block)
                .await
                .map_err(|err| {
                    tracing::error!("failed to encode block: {:?}", err);
                    OperationError::Other("failed to encode block")
                })?;

            let cid = active_block
                .cid()
                .map_err(|_| OperationError::Other("unable to access block cid"))?;

            store.store(cid.clone(), sealed_block, false).await?;

            let locations = content_indexes
                .iter()
                .map(|i| ContentLocation::data(cids[*i].clone(), *i as u64))
                .collect::<Vec<_>>();
            content_indexes.clear();

            let content_ref = ContentReference::new(cid, active_block.data_options(), locations);
            content_references.push(content_ref);
        }

        let locked_key = node_data_key
            .lock_with(rng, &data_key)
            .map_err(|_| OperationError::Other("failed to seal node data key"))?;

        let mut inner_write = self.inner.write().await;
        let node = inner_write.by_perm_id_mut(&new_permanent_id).await?;

        #[cfg(feature = "mime-type")]
        set_mime_type(data, node).await;

        let node_data = node.data_mut().await;
        let file_content =
            FileContent::encrypted(locked_key, plaintext_cid, data_size, content_references);
        *node_data = NodeData::full_file(file_content);

        inner_write.clean_drive().await?;
        Ok(())
    }
}

// todo: should these operations be using the permanent ids? Is that worth the extra
// level of indirection? As long as we remain consistent it should be fine.
#[instrument(level = Level::TRACE, skip(inner, path))]
fn walk_path<'a>(
    inner: &'a Arc<RwLock<InnerDrive>>,
    working_directory_id: NodeId,
    path: &'a [&'a str],
    depth: usize,
) -> BoxFuture<'a, Result<WalkState<'a>, OperationError>> {
    trace!("directory::walk_directory");

    async move {
        let inner_read = inner.read().await;

        let (raw_child_name, remaining_path) = match path.split_first() {
            Some(pair) => pair,
            // We've reached the end of the path, our current node is the target
            None => return Ok(WalkState::found(working_directory_id)),
        };

        let child_name = NodeName::try_from(*raw_child_name)?;
        let current_node = inner_read.by_id(working_directory_id)?;

        let child_map = match current_node.data() {
            NodeData::Directory { children, .. } => children,
            NodeData::File { associated_data, .. } => associated_data,
            _ => return Err(OperationError::NotTraversable),
        };

        let perm_id = match child_map.get(&child_name) {
            Some(entry) => entry.permanent_id(),
            None => {
                return Ok(WalkState::MissingComponent {
                    working_directory_id,
                    missing_name: child_name,
                    remaining_path,
                });
            }
        };

        let next_node = inner_read.by_perm_id(perm_id)?;
        let next_node_id = next_node.id();
        trace!(node_id = ?next_node_id, next_node_kind = ?next_node.kind(), "drive::walk_directory::next_node");

        if !next_node.supports_children() {
            return Err(OperationError::NotTraversable);
        }

        drop(inner_read);

        if depth >= MAX_PATH_DEPTH {
            return Err(OperationError::PathTooDeep);
        }

        walk_path(inner, next_node_id, remaining_path, depth + 1).await
    }
    .boxed()
}

#[cfg(feature = "mime-type")]
async fn set_mime_type(data: &[u8], node: &mut Node) {
    if let Some(mime_type) = MimeGuesser::default()
        .with_name(node.name().clone())
        .with_data(data)
        .guess_mime_type()
    {
        node.set_attribute(MetadataKey::MimeType, mime_type.to_string().into())
            .await;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::filesystem::drive::inner::test::build_interesting_inner;
    #[cfg(feature = "mime-type")]
    use crate::prelude::MemoryDataStore;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn mv_dir_from_dir_to_cwd_specify_name() {
        let mut rng = crate::utils::crypto_rng();
        let mut handle = interesting_handle(None).await;
        handle
            .mv(&mut rng, &["dir_1", "dir_2"], &["dir_2_new"])
            .await
            .unwrap();

        let cwd_ls = handle.ls(&[]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from("dir_2_new").unwrap())
                .count(),
            1
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn mv_dir_from_dir_to_dir_specify_name() {
        let mut rng = crate::utils::crypto_rng();
        let mut handle = interesting_handle(None).await;
        handle
            .mv(
                &mut rng,
                &["dir_1", "dir_2", "dir_3"],
                &["dir_1", "dir_3_new"],
            )
            .await
            .unwrap();

        let cwd_ls = handle.ls(&["dir_1"]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from("dir_3_new").unwrap())
                .count(),
            1
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn mv_file_from_dir_to_cwd_specify_name() {
        let mut rng = crate::utils::crypto_rng();
        let mut handle = interesting_handle(None).await;
        handle
            .mv(
                &mut rng,
                &["dir_1", "dir_2", "dir_3", "file_3"],
                &["file_3_new"],
            )
            .await
            .unwrap();

        let cwd_ls = handle.ls(&[]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from("file_3_new").unwrap())
                .count(),
            1
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn mv_file_from_dir_to_dir_specify_name() {
        let mut rng = crate::utils::crypto_rng();
        let mut handle = interesting_handle(None).await;
        handle
            .mv(
                &mut rng,
                &["dir_1", "dir_2", "dir_3", "file_3"],
                &["dir_1", "file_3_new"],
            )
            .await
            .unwrap();

        let cwd_ls = handle.ls(&["dir_1"]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from("file_3_new").unwrap())
                .count(),
            1
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn mv_dir_from_dir_to_cwd_no_name() {
        let mut rng = crate::utils::crypto_rng();
        let mut handle = interesting_handle(None).await;
        handle.mv(&mut rng, &["dir_1", "dir_2"], &[]).await.unwrap();

        let cwd_ls = handle.ls(&[]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from("dir_2").unwrap())
                .count(),
            1
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn mv_dir_from_dir_to_dir_no_name() {
        let mut rng = crate::utils::crypto_rng();
        let mut handle = interesting_handle(None).await;
        handle
            .mv(&mut rng, &["dir_1", "dir_2", "dir_3"], &["dir_1"])
            .await
            .unwrap();

        let cwd_ls = handle.ls(&["dir_1"]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from("dir_3").unwrap())
                .count(),
            1
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn mv_file_from_dir_to_cwd_no_name() {
        let mut rng = crate::utils::crypto_rng();
        let mut handle = interesting_handle(None).await;
        handle
            .mv(&mut rng, &["dir_1", "dir_2", "dir_3", "file_3"], &[])
            .await
            .unwrap();

        let cwd_ls = handle.ls(&[]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from("file_3").unwrap())
                .count(),
            1
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn mv_file_from_dir_to_dir_no_name() {
        let mut rng = crate::utils::crypto_rng();
        let mut handle = interesting_handle(None).await;
        handle
            .mv(&mut rng, &["dir_1", "dir_2", "dir_3", "file_3"], &["dir_1"])
            .await
            .unwrap();

        let cwd_ls = handle.ls(&["dir_1"]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from("file_3").unwrap())
                .count(),
            1
        );
    }

    async fn interesting_handle(current_key: Option<SigningKey>) -> DirectoryHandle {
        //           -----file_1
        //         /
        // root ---------file_2
        //         \
        //          --------- dir_1 ----- dir_2 ---- dir_3 ---- file_3
        //                         \
        //                           --- file_4
        //                            \
        //                              ----file_5
        let mut rng = crate::utils::crypto_rng();
        let inner = build_interesting_inner(current_key.clone()).await;
        let root_id = inner.root_node().unwrap().id();
        let inner = Arc::new(RwLock::new(inner));
        DirectoryHandle {
            current_key: Arc::new(current_key.unwrap_or_else(|| SigningKey::generate(&mut rng))),
            inner,
            cwd_id: root_id,
        }
    }

    #[cfg(feature = "mime-type")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn sniff_html_mime_type() {
        let mut rng = crate::utils::crypto_rng();
        let current_key = SigningKey::generate(&mut rng);
        let mut handle = interesting_handle(Some(current_key)).await;
        let mut store = MemoryDataStore::default();

        let test_cases = vec![
            (b"<html><head><title>Test File</title></head><body><h1>Hello World!</h1></body></html>".to_vec(), "test.html"),
            (b"<HTML><HEAD><TITLE>Test File</TITLE></HEAD><BODY><H1>Hello World!</H1></BODY></HTML>".to_vec(), "TEST.HTML"),
            (b"<h1>Heading</h1><p>Paragraph</p>".to_vec(), "file.htm"),
            (b"<div><span>Some text</span></div>".to_vec(), "page.php"),
            (
                b"<!docTYPE html><html><body>Content</body></html>".to_vec(),
                "invalid_file_name",
            ),
        ];
        for (data, file_name) in test_cases {
            handle
                .write(&mut rng, &mut store, &[file_name], &data)
                .await
                .unwrap();

            let cwd_ls = handle.ls(&[]).await.unwrap();
            assert_eq!(
                cwd_ls
                    .iter()
                    .filter(|entry| entry.name() == NodeName::try_from(file_name).unwrap())
                    .count(),
                1
            );

            let file_entry = cwd_ls
                .iter()
                .find(|entry| entry.name() == NodeName::try_from(file_name).unwrap())
                .unwrap();

            assert_eq!(file_entry.kind(), NodeKind::File);

            let file_data = handle.read(&mut store, &[file_name]).await.unwrap();
            assert_eq!(file_data.as_slice(), data);

            let mime_type = file_entry.mime_type().unwrap();
            assert_eq!(mime_type, "text/html");
        }
    }

    #[cfg(feature = "mime-type")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn sniff_mp3_file_mime_type() {
        let mut rng = crate::utils::crypto_rng();
        let current_key = SigningKey::generate(&mut rng);
        let mut handle = interesting_handle(Some(current_key)).await;
        let mut store = MemoryDataStore::default();
        let mp3_test_case: &[u8] = &[
            0x49, 0x44, 0x33, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x22, 0x54, 0x53, 0x53, 0x45,
            0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x03, 0x4c, 0x61, 0x76, 0x66, 0x36, 0x30, 0x2e,
            0x33, 0x2e, 0x31, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xff, 0xfb, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let file_name = "the_audio.mp4";
        handle
            .write(&mut rng, &mut store, &[file_name], mp3_test_case)
            .await
            .unwrap();

        let cwd_ls = handle.ls(&[]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from(file_name).unwrap())
                .count(),
            1
        );

        let file_entry = cwd_ls
            .iter()
            .find(|entry| entry.name() == NodeName::try_from(file_name).unwrap())
            .unwrap();

        assert_eq!(file_entry.kind(), NodeKind::File);

        let file_data = handle.read(&mut store, &[file_name]).await.unwrap();
        assert_eq!(file_data.as_slice(), mp3_test_case);

        let mime_type = file_entry.mime_type().unwrap();
        assert_eq!(mime_type, "audio/mpeg");
    }

    #[cfg(feature = "mime-type")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn sniff_mp4_file_mime_type() {
        let mut rng = crate::utils::crypto_rng();
        let current_key = SigningKey::generate(&mut rng);
        let mut handle = interesting_handle(Some(current_key)).await;
        let mut store = MemoryDataStore::default();
        let mp4_test_case: &[u8] = &[
            0x00, 0x00, 0x00, 0x1c, 0x66, 0x74, 0x79, 0x70, 0x69, 0x73, 0x6f, 0x6d, 0x00, 0x00,
            0x02, 0x00, 0x69, 0x73, 0x6f, 0x6d, 0x69, 0x73, 0x6f, 0x32, 0x6d, 0x70, 0x34, 0x31,
            0x00, 0x00, 0x00, 0x08,
        ];
        let file_name = "the_audio.mp3";
        handle
            .write(&mut rng, &mut store, &[file_name], mp4_test_case)
            .await
            .unwrap();

        let cwd_ls = handle.ls(&[]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from(file_name).unwrap())
                .count(),
            1
        );

        let file_entry = cwd_ls
            .iter()
            .find(|entry| entry.name() == NodeName::try_from(file_name).unwrap())
            .unwrap();

        assert_eq!(file_entry.kind(), NodeKind::File);

        let file_data = handle.read(&mut store, &[file_name]).await.unwrap();
        assert_eq!(file_data.as_slice(), mp4_test_case);

        let mime_type = file_entry.mime_type().unwrap();
        assert_eq!(mime_type, "video/mp4");
    }

    #[cfg(feature = "mime-type")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn sniff_webm_file_mime_type() {
        let mut rng = crate::utils::crypto_rng();
        let current_key = SigningKey::generate(&mut rng);
        let mut handle = interesting_handle(Some(current_key)).await;
        let mut store = MemoryDataStore::default();
        let webm_test_case: &[u8] = &[
            0x1a, 0x45, 0xdf, 0xa3, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1f, 0x42, 0x86,
            0x81, 0x01, 0x42, 0xf7, 0x81, 0x01, 0x42, 0xf2, 0x81, 0x04, 0x42, 0xf3, 0x81, 0x08,
            0x42, 0x82, 0x84, 0x77, 0x65, 0x62, 0x6d, 0x42, 0x87, 0x81, 0x02, 0x42, 0x85, 0x81,
            0x02, 0x18, 0x53, 0x80, 0x67, 0x01, 0x00, 0x00, 0x00, 0x00, 0x0d, 0xc0, 0x0a, 0x11,
            0x4d, 0x9b, 0x74, 0x40, 0x3c, 0x4d, 0xbb, 0x8b, 0x53, 0xab, 0x84, 0x15, 0x49, 0xa9,
            0x66, 0x53, 0xac, 0x81, 0xe5, 0x4d, 0xbb, 0x8c, 0x53, 0xab,
        ];
        let file_name = "the_audio.mp4";
        handle
            .write(&mut rng, &mut store, &[file_name], webm_test_case)
            .await
            .unwrap();

        let cwd_ls = handle.ls(&[]).await.unwrap();
        assert_eq!(
            cwd_ls
                .iter()
                .filter(|entry| entry.name() == NodeName::try_from(file_name).unwrap())
                .count(),
            1
        );

        let file_entry = cwd_ls
            .iter()
            .find(|entry| entry.name() == NodeName::try_from(file_name).unwrap())
            .unwrap();

        assert_eq!(file_entry.kind(), NodeKind::File);

        let file_data = handle.read(&mut store, &[file_name]).await.unwrap();
        assert_eq!(file_data.as_slice(), webm_test_case);

        let mime_type = file_entry.mime_type().unwrap();
        assert_eq!(mime_type, "video/webm");
    }
}
