use std::collections::HashMap;
use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::AsyncWrite;
use slab::Slab;

use crate::codec::crypto::*;
use crate::codec::header::*;
use crate::codec::*;
use crate::filesystem::nodes::NodeKind;
use crate::filesystem::operations::*;
use crate::filesystem::{DriveAccess, Node, NodeBuilder, NodeId};

pub struct Drive {
    current_key: Arc<SigningKey>,
    filesystem_id: FilesystemId,
    // todo: need to switch to a mutex, can't have state being modified during an encoding session
    // and the cooperative multitasking model of async/await means we can't guarantee that some
    // other task isn't going to tweak it
    inner: Arc<RwLock<InnerDrive>>,
}

struct InnerDrive {
    access: DriveAccess,
    nodes: Slab<Node>,
    root_node_id: NodeId,
    permanent_id_map: HashMap<PermanentId, NodeId>,
}

impl Drive {
    pub async fn encode_private<W: AsyncWrite + Unpin + Send>(
        &self,
        _rng: &mut impl CryptoRngCore,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += IdentityHeader::encode(&IdentityHeader, writer).await?;
        written_bytes += self.filesystem_id.encode(writer).await?;

        // Don't support ECC yet
        written_bytes += PublicSettings::new(false, true).encode(writer).await?;

        Ok(written_bytes)
    }

    pub async fn has_realized_view_access(&self, actor_id: ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access.has_realized_view_access(actor_id)
    }

    pub async fn has_write_access(&self, actor_id: ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access.has_write_access(actor_id)
    }

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
        path: &[&str],
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
    Found(NodeId, &'a str),

    /// If some or all of the path has been walked, but ran out of named nodes that match the path.
    /// It returns the ID of the last directory it was able to walk to and the missing part of the
    /// path.
    Missing(NodeId, &'a [&'a str]),

    /// Part of the provided path was not a directory so traversal was stopped. The last valid
    /// directory ID and the remaining path is returned.
    NotTraversable(NodeId, &'a [&'a str]),
}

impl Directory {
    // todo: these operations should really be using the permanent ids
    async fn walk_directory<'b>(
        &self,
        mut cwd_id: NodeId,
        mut path: &'b [&'b str],
    ) -> Result<WalkState<'b>, OperationError> {
        loop {
            let (current_entry, remaining_path) = match path.split_first() {
                Some(r) => r,
                None => {
                    return Err(OperationError::UnexpectedEmptyPath);
                }
            };

            match *current_entry {
                // This path references where we currently are, do nothing and let the next loop
                // advance our state
                "." => (),
                // Zip up the directory level
                ".." => match self.inner.read().await.nodes[cwd_id].parent_id() {
                    Some(parent_id) => {
                        cwd_id = parent_id;
                    }
                    None => return Err(OperationError::InvalidParentDir),
                },
                cur_name => {
                    if cur_name.is_empty() {
                        return Err(OperationError::NameIsEmpty);
                    }

                    if cur_name.len() > 255 {
                        return Err(OperationError::PathComponentTooLong);
                    }

                    let inner_read = self.inner.read().await;
                    let node_children = match inner_read.nodes[cwd_id].kind() {
                        NodeKind::Directory { children, .. } => children,
                        _ => {
                            return Err(OperationError::IncompatibleType(
                                "current node is not a directory",
                            ))
                        }
                    };

                    let perm_id = match node_children.get(cur_name) {
                        Some(perm_id) => perm_id,
                        None => return Ok(WalkState::Missing(cwd_id, path)),
                    };

                    let next_cwd_id = inner_read
                        .permanent_id_map
                        .get(perm_id)
                        .ok_or(OperationError::MissingPermanentId(*perm_id))?;

                    if path.is_empty() {
                        return Ok(WalkState::Found(*next_cwd_id, cur_name));
                    }

                    // The path goes deeper, make sure the next node is a directory
                    let next_node = &inner_read.nodes[*next_cwd_id];
                    if !matches!(next_node.kind(), NodeKind::Directory { .. }) {
                        return Ok(WalkState::NotTraversable(cwd_id, path));
                    }

                    // Go deeper on our next loop, importantly though an empty path from this point on
                    // means we've successfully reached our goal.
                    cwd_id = *next_cwd_id;
                    path = remaining_path;
                }
            }
        }
    }

    pub async fn cd(&self, path: &[&str]) -> Result<Directory, OperationError> {
        let target_directory_id = match self.walk_directory(self.cwd_id, path).await {
            Ok(WalkState::Found(tdi_id, _)) => tdi_id,
            _ => return Err(OperationError::NotADirectory),
        };

        tracing::debug!("drive::cd::{{path:?}}");

        let directory = Directory {
            current_key: self.current_key.clone(),
            cwd_id: target_directory_id,
            inner: self.inner.clone(),
        };

        Ok(directory)
    }

    pub async fn ls(self, path: &[&str]) -> Result<Vec<(String, PermanentId)>, OperationError> {
        let (target_dir_id, entry) = match self.walk_directory(self.cwd_id, path).await? {
            WalkState::Found(tdi_id, parent_entry) => (tdi_id, parent_entry),
            // If the last item is a file we can display it, this matches the behavior in linux
            // like shells
            WalkState::NotTraversable(tdi, blocked_path) if blocked_path.len() == 1 => {
                (tdi, blocked_path[0])
            }
            WalkState::NotTraversable(_, _) => return Err(OperationError::NotADirectory),
            WalkState::Missing(_, _) => return Err(OperationError::PathNotFound),
        };

        let inner_read = self.inner.read().await;
        let target_dir = &inner_read.nodes[target_dir_id];

        let node_children = match target_dir.kind() {
            NodeKind::Directory { children, .. } => children,
            NodeKind::File { .. } => {
                return Ok(vec![(entry.to_string(), target_dir.permanent_id())]);
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
        mut path: &[&str],
        recursive: bool,
    ) -> Result<(), OperationError> {
        let mut cwd_id = self.cwd_id;
        tracing::debug!(initial_working_directory_id = ?cwd_id, "drive::mkdir::{{{path:?}}}");

        if path.is_empty() {
            return Err(OperationError::UnexpectedEmptyPath);
        }

        loop {
            match self.walk_directory(cwd_id, path).await? {
                // node already exists, we'll double check its a folder and consider it a success
                // if it is
                WalkState::Found(nid, entry_name) => {
                    match self.inner.read().await.nodes[nid].kind() {
                        NodeKind::Directory { .. } => {
                            tracing::debug!(directory = entry_name, "drive::mkdir::already_exists");
                            return Ok(());
                        }
                        _ => return Err(OperationError::NotADirectory),
                    }
                }
                WalkState::NotTraversable(_, _) => return Err(OperationError::NotADirectory),
                WalkState::Missing(current_dir_id, missing_path) if !missing_path.is_empty() => {
                    // we should always have
                    let (missing_name, remaining_path) = match missing_path.split_first() {
                        Some(res) => res,
                        _ => unreachable!("protected by branch guard"),
                    };

                    tracing::debug!(cwd_id = ?current_dir_id, name = ?missing_name, "drive::mkdir::missing_directory");

                    // If there is more to the path and we are not in recursive mode, we should
                    // report the error
                    if !(recursive || remaining_path.is_empty()) {
                        tracing::debug!("drive::mkdir::not_recursive");
                        return Err(OperationError::PathNotFound);
                    }

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

                    let node_children = match inner_write.nodes[current_dir_id].kind_mut() {
                        NodeKind::Directory { children, .. } => children,
                        _ => return Err(OperationError::BadSearch("not in a directory?")),
                    };

                    node_children.insert(missing_name.to_string(), new_permanent_id);
                    tracing::debug!(
                        cwd_id = ?current_dir_id,
                        name = missing_name,
                        pid = ?new_permanent_id,
                        "drive::mkdir::created"
                    );

                    if remaining_path.is_empty() {
                        // Nothing left to do, let's bail out
                        return Ok(());
                    }

                    cwd_id = new_node_id;
                    path = remaining_path;
                }
                // This shouldn't happen as this branch should be considered "Found" posibly an
                // empty path error, either way we shouldn't be here
                WalkState::Missing(_, _) => {
                    return Err(OperationError::BadSearch("unexpected lack of directory"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_creating_directories() {
        todo!()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_walking_relative_directories() {
        let mut rng = crate::utils::crypto_rng();

        let current_key = SigningKey::generate(&mut rng);
        let _drive = Drive::initialize_private(&mut rng, current_key);

        todo!()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_walking_absolute_directories() {
        todo!()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_missing_child_in_path_traversal() {
        todo!()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_non_directory_in_path_traversal() {
        todo!()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_parent_traversal() {
        todo!()
    }
}
