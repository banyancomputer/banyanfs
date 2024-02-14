use std::collections::HashMap;
use std::path::{Component, Path};
use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use slab::Slab;

use crate::codec::crypto::SigningKey;
use crate::codec::header::KeyAccessSettingsBuilder;
use crate::codec::meta::{ActorId, FilesystemId};
use crate::filesystem::nodes::NodeKind;
use crate::filesystem::operations::*;
use crate::filesystem::{DriveAccess, Node, NodeBuilder, NodeId, PermanentId};

pub struct Drive {
    current_key: Arc<SigningKey>,
    filesystem_id: FilesystemId,
    inner: Arc<RwLock<InnerDrive>>,
}

struct InnerDrive {
    access: DriveAccess,
    nodes: Slab<Node>,
    root_node_id: NodeId,
    permanent_id_map: HashMap<PermanentId, NodeId>,
}

impl Drive {
    pub async fn has_realized_view_access(&self, actor_id: ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access.has_realized_view_access(actor_id)
    }

    pub async fn has_write_access(&self, actor_id: ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access.has_write_access(actor_id)
    }

    //pub async fn encode_private<W: AsyncWrite + Unpin + Send>(
    //    &self,
    //    rng: &mut impl CryptoRngCore,
    //    writer: &mut W,
    //    _signing_key: &SigningKey,
    //) -> std::io::Result<usize> {
    //    let mut written_bytes = 0;

    //    written_bytes += IdentityHeader::encode(&IdentityHeader, writer).await?;
    //    written_bytes += self.filesystem_id.encode(writer).await?;

    //    // Don't support ECC yet
    //    written_bytes += PublicSettings::new(false, true).encode(writer).await?;

    //    let encoding_context = PrivateEncodingContext::new(
    //        rng,
    //        self.keys.clone(),
    //        (0, 0),
    //        (Cid::from([0u8; 32]), Cid::from([0u8; 32])),
    //    );

    //    let content_payload = ContentPayload::Private;
    //    written_bytes += content_payload
    //        .encode_private(rng, &encoding_context, writer)
    //        .await?;

    //    Ok(written_bytes)
    //}

    pub fn id(&self) -> FilesystemId {
        self.filesystem_id
    }

    pub fn initialize_private(rng: &mut impl CryptoRngCore, current_key: SigningKey) -> Self {
        let verifying_key = current_key.verifying_key();
        let actor_id = verifying_key.actor_id();

        let filesystem_id = FilesystemId::generate(rng);
        tracing::debug!(?actor_id, ?filesystem_id, "drive::initializing_private");

        let kas = KeyAccessSettingsBuilder::private()
            .set_owner()
            .set_protected()
            .with_all_access()
            .build();

        let mut access = DriveAccess::default();
        access.register_actor(verifying_key, kas);

        let mut nodes = Slab::with_capacity(32);
        let mut permanent_id_map = HashMap::new();

        let node_entry = nodes.vacant_entry();
        let root_node_id = node_entry.key();

        let directory = NodeBuilder::directory(root_node_id, actor_id).build(rng);
        permanent_id_map.insert(directory.permanent_id(), root_node_id);
        node_entry.insert(directory);

        let current_key = Arc::new(current_key);

        Self {
            current_key,
            filesystem_id,

            inner: Arc::new(RwLock::new(InnerDrive {
                access,
                nodes,
                root_node_id,
                permanent_id_map,
            })),
        }
    }

    pub async fn mkdir(
        &mut self,
        rng: &mut impl CryptoRngCore,
        path: &Path,
        recursive: bool,
    ) -> Result<(), OperationError> {
        self.root_directory()
            .await
            .mkdir(rng, path, recursive)
            .await
    }

    pub async fn root_directory(&mut self) -> Directory {
        let inner_read = self.inner.read().await;
        let root_node_id = inner_read.root_node_id;
        drop(inner_read);

        Directory::new(self.current_key.clone(), root_node_id, self.inner.clone()).await
    }
}

pub struct Directory {
    current_key: Arc<SigningKey>,
    cwd_id: NodeId,
    inner: Arc<RwLock<InnerDrive>>,
}

enum WalkState<'a> {
    /// The path was fully traversed and resulted in the included ID and the final part of the
    /// path that matched
    Found(NodeId, Component<'a>),

    /// If some or all of the path has been walked, but ran out of named nodes that match the path.
    /// It returns the ID of the last directory it was able to walk to and the missing part of the
    /// path.
    Missing(NodeId, &'a Path),

    /// Part of the provided path was not a directory so traversal was stopped. The last valid
    /// directory ID and the remaining path is returned.
    NotTraversable(NodeId, &'a Path),
}

impl Directory {
    // todo: these operations should really be using the permanent ids
    async fn walk_directory<'b>(
        &self,
        mut cwd_id: NodeId,
        path: &'b Path,
    ) -> Result<WalkState<'b>, OperationError> {
        let mut components = path.components();

        // For the first one, if its empty the path is invalid, later on it means we'll have
        // reached our destination
        let mut active_path = match components.next() {
            Some(path) => path,
            None => return Err(OperationError::UnexpectedEmptyPath),
        };

        loop {
            match active_path {
                Component::RootDir => {
                    cwd_id = self.inner.read().await.root_node_id;
                }
                // This path references where we currently are, do nothing and let the next loop
                // advance our state
                Component::CurDir => (),
                // Weird windows prefixes, ignore them
                Component::Prefix(_) => (),
                // Zip up the directory level
                Component::ParentDir => match self.inner.read().await.nodes[cwd_id].parent_id() {
                    Some(parent_id) => {
                        cwd_id = parent_id;
                    }
                    None => return Err(OperationError::InvalidParentDir),
                },
                Component::Normal(current_path_os) => {
                    let current_path = current_path_os
                        .to_str()
                        .ok_or(OperationError::InvalidPath)?;

                    if current_path.is_empty() {
                        return Err(OperationError::UnexpectedEmptyPath);
                    }

                    if current_path.len() > 255 {
                        return Err(OperationError::PathComponentTooLong);
                    }

                    let inner_read = self.inner.read().await;
                    let node_children = match inner_read.nodes[cwd_id].kind() {
                        NodeKind::Directory { children, .. } => children,
                        _ => return Err(OperationError::IncompatibleType("ls")),
                    };

                    let perm_id = match node_children.get(current_path) {
                        Some(perm_id) => perm_id,
                        None => return Ok(WalkState::Missing(cwd_id, components.as_path())),
                    };

                    let next_cwd_id = inner_read
                        .permanent_id_map
                        .get(perm_id)
                        .ok_or(OperationError::MissingPermanentId(*perm_id))?;

                    match inner_read.nodes[*next_cwd_id].kind() {
                        // We can add in links, and external mounted filesystems here later on
                        NodeKind::Directory { .. } => {
                            cwd_id = *next_cwd_id;
                        }
                        _ => return Ok(WalkState::NotTraversable(cwd_id, components.as_path())),
                    }
                }
            }

            active_path = match components.next() {
                Some(path) => path,
                None => return Ok(WalkState::Found(cwd_id, active_path)),
            };
        }
    }

    pub async fn cd(mut self, path: &Path) -> Result<(), OperationError> {
        let target_directory_id = match self.walk_directory(self.cwd_id, path).await {
            Ok(WalkState::Found(tdi_id, _)) => tdi_id,
            _ => return Err(OperationError::NotADirectory),
        };

        self.cwd_id = target_directory_id;

        Ok(())
    }

    pub async fn ls(self, path: &Path) -> Result<Vec<(String, PermanentId)>, OperationError> {
        let (target_directory_id, remaining) = match self.walk_directory(self.cwd_id, path).await? {
            WalkState::Found(tdi_id, component) => (tdi_id, component),
            // If the last item is a file we can display it, this matches the behavior in linux
            // like shells
            WalkState::NotTraversable(tdi, path) if path.components().count() == 1 => {
                (tdi, path.components().next().unwrap())
            }
            WalkState::NotTraversable(_, _) => return Err(OperationError::NotADirectory),
            WalkState::Missing(_, _) => return Err(OperationError::PathNotFound),
        };

        let inner_read = self.inner.read().await;
        let node_children = match inner_read.nodes[target_directory_id].kind() {
            NodeKind::Directory { children, .. } => children,
            NodeKind::File { .. } => {
                let name = remaining
                    .as_os_str()
                    .to_str()
                    .ok_or(OperationError::InvalidPath)?;

                return Ok(vec![(
                    name.to_string(),
                    inner_read.nodes[target_directory_id].permanent_id(),
                )]);
            }
        };

        let children = node_children.iter().map(|(k, v)| (k.clone(), *v)).collect();

        Ok(children)
    }

    async fn new(
        current_key: Arc<SigningKey>,
        cwd_id: NodeId,
        inner: Arc<RwLock<InnerDrive>>,
    ) -> Self {
        let inner_read = inner.read().await;

        debug_assert!(inner_read.nodes.contains(cwd_id));
        debug_assert!(matches!(
            inner_read.nodes[cwd_id].kind(),
            NodeKind::Directory { .. }
        ));
        drop(inner_read);

        Self {
            current_key,
            cwd_id,
            inner,
        }
    }

    pub async fn mkdir(
        &mut self,
        rng: &mut impl CryptoRngCore,
        path: &Path,
        recursive: bool,
    ) -> Result<(), OperationError> {
        let mut cwd_id = self.cwd_id;
        let mut path_components = path.components();

        loop {
            match self
                .walk_directory(cwd_id, path_components.as_path())
                .await?
            {
                // directory already exists, don't need to do anything
                WalkState::Found(_, _) => return Ok(()),
                WalkState::NotTraversable(_, _) => return Err(OperationError::NotADirectory),
                WalkState::Missing(containing_node_id, remaining_path) => {
                    path_components = remaining_path.components();

                    let potential_directory_name = match path_components.next() {
                        Some(name) => name,
                        None => unreachable!("should be caught by Found branch"),
                    };

                    // If there is more to the path and we are not in recursive mode, we should
                    // report the error
                    if !recursive && path_components.by_ref().peekable().peek().is_some() {
                        return Err(OperationError::PathNotFound);
                    }

                    let directory_name = potential_directory_name
                        .as_os_str()
                        .to_str()
                        .ok_or(OperationError::InvalidPath)?;

                    tracing::info!(orig_cwd = ?self.cwd_id, now_cwd = ?cwd_id, "creating directory '{potential_directory_name:?}'");

                    // Create our new directory and set it up within the slab, nothing has been
                    // persisted yet and nothing about it will be recorded as we don't have a
                    // concept of its permenant ID
                    let mut inner_write = self.inner.write().await;
                    let node_entry = inner_write.nodes.vacant_entry();
                    let new_node_id = node_entry.key();

                    let actor_id = self.current_key.actor_id();
                    let new_directory = NodeBuilder::directory(new_node_id, actor_id).build(rng);
                    let new_permanent_id = new_directory.permanent_id();
                    node_entry.insert(new_directory);

                    // The lock requires us to get a new lock handle to mutate another portion
                    drop(inner_write);
                    let mut inner_write = self.inner.write().await;

                    // We register the new directory in our pemranent ID map, this will persist it
                    // but its still a loose leaf in the graph. We need to inform its parent
                    // that it has a new child
                    inner_write
                        .permanent_id_map
                        .insert(new_permanent_id, new_node_id);

                    // ...which necessitates a new write
                    drop(inner_write);
                    let mut inner_write = self.inner.write().await;
                    let node_children = match inner_write.nodes[containing_node_id].kind_mut() {
                        NodeKind::Directory { children, .. } => children,
                        _ => {
                            return Err(OperationError::BadSearch(
                                "only directories should be found here",
                            ))
                        }
                    };

                    node_children.insert(directory_name.to_string(), new_permanent_id);

                    cwd_id = new_node_id;
                }
            };
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    //    #[error("failed to parse drive data, is this a banyanfs file?")]
    //    HeaderReadFailure,
}
