use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use futures::future::BoxFuture;
use futures::FutureExt;
use tracing::{debug, instrument, trace, Instrument, Level};

use crate::codec::filesystem::NodeKind;
use crate::codec::*;

use crate::codec::crypto::{AccessKey, SigningKey};
use crate::codec::filesystem::BlockKind;
use crate::codec::header::DataBlock;
use crate::filesystem::drive::{DirectoryEntry, InnerDrive, OperationError, WalkState};
use crate::filesystem::nodes::{Node, NodeData, NodeId, NodeName};
use crate::filesystem::{ContentLocation, ContentReference, FileContent, NodeBuilder};
use crate::stores::DataStore;

use self::filesystem::FilePermissions;

const MAX_PATH_DEPTH: usize = 32;

#[derive(Clone)]
pub struct DirectoryHandle {
    pub(crate) current_key: Arc<SigningKey>,
    pub(crate) cwd_id: NodeId,
    pub(crate) inner: Arc<RwLock<InnerDrive>>,
}

impl DirectoryHandle {
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

    pub async fn chmod(
        &self,
        _path: &[&str],
        _owner: FilePermissions,
    ) -> Result<(), OperationError> {
        unimplemented!()
    }

    pub async fn chown(&self, _path: &[&str], _owner: ActorId) -> Result<(), OperationError> {
        unimplemented!()
    }

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

        for perm_id in children.into_iter() {
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
        let src_node_id = match walk_path(&self.inner, self.cwd_id, src_path, 0).await? {
            WalkState::FoundNode { node_id } => node_id,
            WalkState::MissingComponent { .. } => return Err(OperationError::PathNotFound),
        };

        let dst_parent_id = match walk_path(&self.inner, self.cwd_id, dst_path, 0).await? {
            WalkState::FoundNode { node_id } => {
                let inner_read = self.inner.read().await;
                let found_node = inner_read.by_id(node_id)?;

                if !found_node.supports_children() {
                    return Err(OperationError::Exists(node_id));
                }

                node_id
            }
            WalkState::MissingComponent {
                working_directory_id,
                remaining_path,
                ..
            } => {
                if !remaining_path.is_empty() {
                    return Err(OperationError::PathNotFound);
                }

                working_directory_id
            }
        };

        let mut inner_write = self.inner.write().await;

        //todo(sstelfox): update this to use the new node manipulation API, might allow us to
        //remove the data() data_mut() functions entirely...
        let src_node = inner_write.by_id_mut(src_node_id)?;
        let src_parent_perm_id = src_node
            .parent_id()
            .ok_or(OperationError::InternalCorruption(
                src_node_id,
                "src node has no parent",
            ))?;

        let src_name = src_node.name();
        let src_perm_id = src_node.permanent_id();
        src_node.set_parent_id(src_perm_id).await;

        let src_parent_node = inner_write.by_perm_id_mut(&src_parent_perm_id)?;
        match src_parent_node.data_mut().await {
            NodeData::Directory { children, .. } => children.remove(&src_name),
            _ => {
                return Err(OperationError::InternalCorruption(
                    src_parent_node.id(),
                    "source node parent is not a directory",
                ));
            }
        };

        let dst_parent_node = inner_write.by_id_mut(dst_parent_id)?;

        let last_dst_element = dst_path.last().ok_or(OperationError::UnexpectedEmptyPath)?;
        let new_dst_name = NodeName::named(last_dst_element.to_string())?;

        match dst_parent_node.data_mut().await {
            NodeData::Directory { children, .. } => {
                if children.insert(new_dst_name.clone(), src_perm_id).is_some() {
                    return Err(OperationError::InternalCorruption(
                        dst_parent_id,
                        "destination parent already had a node with the same name",
                    ));
                }
            }
            _ => {
                return Err(OperationError::InternalCorruption(
                    dst_parent_id,
                    "destination parent is not a directory",
                ))
            }
        }

        let dst_parent_perm_id = dst_parent_node.permanent_id();
        let tgt_node = inner_write.by_id_mut(src_node_id)?;

        tgt_node.set_parent_id(dst_parent_perm_id).await;
        tgt_node.set_name(new_dst_name).await;

        Ok(())
    }

    #[instrument(level = Level::DEBUG, skip(self, _rng))]
    pub async fn rm(
        &mut self,
        _rng: &mut impl CryptoRngCore,
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

        inner_write.remove_node(target_perm_id).await?;

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
        if !inner_read.access().has_read_access(actor_id) {
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

            let data_key = match inner_read
                .access()
                .permission_keys()
                .and_then(|pk| pk.data.as_ref())
            {
                Some(data_key) => data_key,
                None => return Err(OperationError::AccessDenied),
            };

            let unlocked_key = locked_key
                .unlock(data_key)
                .map_err(|_| OperationError::AccessDenied)?;

            let author_id = read_node.owner_id();
            let verifying_key = inner_read
                .access()
                .actor_key(&author_id)
                .ok_or(OperationError::AccessDenied)?;

            let mut file_data = Vec::new();

            for content_ref in node_content.content_references()? {
                if !store.contains_cid(content_ref.data_block_cid()).await? {
                    return Err(OperationError::BlockUnavailable(
                        content_ref.data_block_cid(),
                    ));
                }

                let data_chunk = store.retrieve(content_ref.data_block_cid()).await?;

                let (_remaining, block) =
                    DataBlock::parse_with_magic(&data_chunk, &unlocked_key, &verifying_key)
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

                    let data = block
                        .get_chunk_data(location.block_index() as usize)
                        .map_err(|err| {
                            tracing::error!("failed to retrieve block chunk: {err:?}");
                            OperationError::BlockCorrupted(content_ref.data_block_cid())
                        })?;

                    file_data.extend_from_slice(data);
                }
            }

            Ok(file_data)
        } else {
            unimplemented!()
        }
    }

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
        if !inner_read.access().has_write_access(actor_id) {
            return Err(OperationError::AccessDenied);
        }

        let data_key = match inner_read
            .access()
            .permission_keys()
            .and_then(|pk| pk.data.as_ref())
        {
            Some(data_key) => data_key.clone(),
            None => return Err(OperationError::AccessDenied),
        };

        drop(inner_read);

        let (parent_path, name) = path.split_at(path.len() - 1);
        let file_name = NodeName::try_from(name[0]).map_err(OperationError::InvalidName)?;

        tracing::info!(?path, ?file_name, "drive::write");

        let parent_id = match walk_path(&self.inner, self.cwd_id, parent_path, 0).await? {
            WalkState::FoundNode { node_id } => node_id,
            WalkState::MissingComponent { .. } => return Err(OperationError::PathNotFound),
        };

        tracing::info!(?parent_id, "drive::write::parent_id");

        let inner_read = self.inner.read().await;
        let parent_node = inner_read.by_id(parent_id)?;
        let parent_perm_id = parent_node.permanent_id();
        drop(inner_read);

        let data_size = data.len() as u64;
        let node_name = file_name.clone();

        // todo(sstelfox): handle the special case of an empty file, it shouldn't be a stub and
        // doesn't need to go through the encoding process.

        let new_permanent_id = self
            .insert_node(
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
            .await?;

        const SMALL_BLOCK_THRESHOLD: usize = DataBlock::SMALL_ENCRYPTED_SIZE * 8;
        let block_creator = if data_size > SMALL_BLOCK_THRESHOLD as u64 {
            || match DataBlock::small() {
                Ok(ab) => Ok(ab),
                Err(err) => {
                    tracing::error!("failed to create data block: {:?}", err);
                    Err(OperationError::Other("data block failed"))
                }
            }
        } else {
            || match DataBlock::standard() {
                Ok(ab) => Ok(ab),
                Err(err) => {
                    tracing::error!("failed to create data block: {:?}", err);
                    Err(OperationError::Other("data block failed"))
                }
            }
        };

        // todo(sstelfox): bit lazy here, should calculate this as I stream it but speed right
        // now...
        let plaintext_cid = crate::utils::calculate_cid(data);

        let mut remaining_data = data;
        let mut active_block = block_creator()?;
        let active_block_chunk_size = active_block.chunk_size();
        let node_data_key = AccessKey::generate(rng);
        let mut content_references = Vec::new();

        while !remaining_data.is_empty() {
            let data_to_read = std::cmp::min(remaining_data.len(), active_block_chunk_size);
            let (chunk_data, next_data) = remaining_data.split_at(data_to_read);
            remaining_data = next_data;

            active_block
                .push_chunk(chunk_data.to_vec())
                .map_err(|err| {
                    tracing::error!("failed to push chunk: {:?}", err);
                    OperationError::Other("expected remaining capacity")
                })?;

            if active_block.is_full() {
                let mut sealed_block = Vec::new();

                let (_, cids) = active_block
                    .encode(rng, &node_data_key, &self.current_key, &mut sealed_block)
                    .await
                    .map_err(|_| OperationError::Other("failed to encode block"))?;

                let block_size = *active_block.data_options().block_size();
                let cid = active_block
                    .cid()
                    .map_err(|_| OperationError::Other("unable to access block cid"))?;

                store.store(cid.clone(), sealed_block, false).await?;

                let locations = cids
                    .into_iter()
                    .enumerate()
                    .map(|(i, cid)| ContentLocation::data(cid, i as u64))
                    .collect::<Vec<_>>();

                let content_ref = ContentReference::new(cid, block_size, locations);
                content_references.push(content_ref);

                active_block = block_creator()?;
            }
        }

        if !active_block.is_empty() {
            // todo(sstelfox): this is duplicated, need to extract it
            let mut sealed_block = Vec::new();

            let (_, cids) = active_block
                .encode(rng, &node_data_key, &self.current_key, &mut sealed_block)
                .await
                .map_err(|_| OperationError::Other("failed to encode block"))?;

            let block_size = *active_block.data_options().block_size();
            let cid = active_block
                .cid()
                .map_err(|_| OperationError::Other("unable to access block cid"))?;

            store.store(cid.clone(), sealed_block, false).await?;

            let locations = cids
                .into_iter()
                .enumerate()
                .map(|(i, cid)| ContentLocation::data(cid, i as u64))
                .collect::<Vec<_>>();

            let content_ref = ContentReference::new(cid, block_size, locations);
            content_references.push(content_ref);
        }

        let locked_key = node_data_key
            .lock_with(rng, &data_key)
            .map_err(|_| OperationError::Other("failed to seal node data key"))?;

        let mut inner_write = self.inner.write().await;
        let node = inner_write.by_perm_id_mut(&new_permanent_id)?;
        let node_data = node.data_mut().await;

        let file_content = FileContent::encrypted(locked_key, plaintext_cid, content_references);
        *node_data = NodeData::full_file(file_content);

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
            Some(pid) => pid,
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
