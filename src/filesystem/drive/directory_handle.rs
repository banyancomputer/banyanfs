use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use futures::future::BoxFuture;
use futures::FutureExt;
use tracing::{debug, instrument, trace, Instrument, Level};

use crate::codec::*;
use crate::filesystem::nodes::NodeKind;

use crate::codec::crypto::SigningKey;
use crate::filesystem::drive::{InnerDrive, OperationError, WalkState};
use crate::filesystem::nodes::{Node, NodeId, NodeName};
use crate::filesystem::NodeBuilder;

const MAX_PATH_DEPTH: usize = 32;

pub struct DirectoryHandle {
    current_key: Arc<SigningKey>,
    cwd_id: NodeId,
    inner: Arc<RwLock<InnerDrive>>,
}

impl DirectoryHandle {
    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn cd(&self, path: &[&str]) -> Result<DirectoryHandle, OperationError> {
        debug!(cwd_id = self.cwd_id, "directory::cd");

        let target_directory_id = if path.is_empty() {
            self.cwd_id
        } else {
            match walk_path(&self.inner, self.cwd_id, path, 0).await {
                Ok(WalkState::FoundNode { node_id }) => node_id,
                _ => return Err(OperationError::NotADirectory),
            }
        };

        let directory = DirectoryHandle {
            current_key: self.current_key.clone(),
            cwd_id: target_directory_id,
            inner: self.inner.clone(),
        };

        Ok(directory)
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn ls(self, path: &[&str]) -> Result<Vec<(NodeName, PermanentId)>, OperationError> {
        debug!(cwd_id = self.cwd_id, "directory::ls");

        // These behaviors are slightly different mostly in the error cases, in the first case we
        // should be in a directory, any other state is an error. In the latter case, we can match
        // a specific node as well as a directory and should always succeed if we can reach the
        // node.
        let inner_read = self.inner.read().await;
        let children = if path.is_empty() {
            match inner_read.nodes[self.cwd_id].kind() {
                NodeKind::Directory { children, .. } => children,
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
                _ => return Err(OperationError::NotADirectory),
            };

            let listed_node = &inner_read.nodes[node_id];

            match listed_node.kind() {
                NodeKind::Directory { children, .. } => children,
                _ => return Ok(vec![(listed_node.name(), listed_node.permanent_id())]),
            }
        };

        let children = children.iter().map(|(k, v)| (k.clone(), *v)).collect();

        Ok(children)
    }

    #[instrument(level = Level::TRACE, skip(current_key, inner))]
    pub(crate) async fn new(
        current_key: Arc<SigningKey>,
        cwd_id: NodeId,
        inner: Arc<RwLock<InnerDrive>>,
    ) -> Self {
        trace!("directory::new");

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

    #[instrument(level = tracing::Level::TRACE, skip_all)]
    async fn insert_node<'a, 'b, R, F, Fut>(
        &'b mut self,
        rng: &'a mut R,
        parent_id: NodeId,
        build_node: F,
    ) -> Result<(NodeId, PermanentId), OperationError>
    where
        R: CryptoRngCore,
        F: FnOnce(&'a mut R, NodeId, NodeId, ActorId) -> Fut,
        Fut: std::future::Future<Output = Result<Node, OperationError>>,
    {
        trace!("directory::insert_node");

        let inner_read = self.inner.read().await;
        let parent_node = &inner_read.nodes[parent_id];
        if !parent_node.is_directory() {
            return Err(OperationError::ParentMustBeDirectory);
        }
        drop(inner_read);

        let mut inner_write = self.inner.write().in_current_span().await;
        let node_entry = inner_write.nodes.vacant_entry();
        let node_id = node_entry.key();

        let owner_id = self.current_key.actor_id();
        let node = build_node(rng, parent_id, node_id, owner_id)
            .in_current_span()
            .await?;

        let name = node.name();
        let permanent_id = node.permanent_id();

        node_entry.insert(node);
        inner_write.permanent_id_map.insert(permanent_id, node_id);

        let parent_node =
            inner_write
                .nodes
                .get_mut(parent_id)
                .ok_or(OperationError::InternalCorruption(
                    parent_id,
                    "expected referenced parent to exist",
                ))?;

        let parent_children = match parent_node.kind_mut() {
            NodeKind::Directory { children, .. } => children,
            _ => {
                return Err(OperationError::InternalCorruption(
                    parent_id,
                    "parent node must be a directory",
                ));
            }
        };

        if parent_children.insert(name, permanent_id).is_some() {
            return Err(OperationError::InternalCorruption(
                parent_id,
                "wrote new directory over existing entry",
            ));
        }

        debug!(?node_id, ?permanent_id, "directory::insert_node::inserted");

        Ok((node_id, permanent_id))
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

                    match inner_read.nodes[node_id].kind() {
                        NodeKind::Directory { .. } => return Ok(()),
                        NodeKind::File { .. } => return Err(OperationError::Exists(node_id)),
                    }
                }
                WalkState::MissingComponent {
                    working_directory_id,
                    missing_name,
                    remaining_path,
                } => {
                    debug!(cwd_id = working_directory_id, name = ?missing_name, "drive::mkdir::node_missing");

                    // When we're not recursing and there are more path components left, we have to
                    // abort early
                    if !recursive && !remaining_path.is_empty() {
                        debug!("drive::mkdir::not_recursive");
                        return Err(OperationError::PathNotFound);
                    }

                    self.insert_node(
                        &mut *rng,
                        working_directory_id,
                        |rng, parent_id, new_node_id, actor_id| async move {
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
                        debug!("drive::mkdir::complete");
                        return Ok(());
                    }
                }
                WalkState::NotTraversable {
                    working_directory_id,
                    blocking_name,
                } => {
                    debug!(cwd_id = working_directory_id, name = ?blocking_name, "drive::mkdir::not_traversable");
                    return Err(OperationError::NotADirectory);
                }
            }
        }

        Err(OperationError::PathTooDeep)
    }
}

// todo: should these operations be using the permanent ids? Is that worth the extra
// level of indirection? As long as we remain consistent it should be fine.
#[instrument(level = Level::TRACE, skip(inner))]
fn walk_path<'a>(
    inner: &'a Arc<RwLock<InnerDrive>>,
    working_directory_id: NodeId,
    path: &'a [&'a str],
    depth: usize,
) -> BoxFuture<'a, Result<WalkState<'a>, OperationError>> {
    trace!("directory::walk_directory");

    async move {
        let inner_read = inner.read().await;
        let current_node = inner_read.nodes.get(working_directory_id).ok_or(
            OperationError::InternalCorruption(working_directory_id, "missing working directory"),
        )?;

        let children = match current_node.kind() {
            NodeKind::Directory { children, .. } => children,
            _ => {
                return Err(OperationError::InternalCorruption(
                    working_directory_id,
                    "current working directory not directory",
                ))
            }
        };

        let (current_entry, remaining_path) = match path.split_first() {
            Some((name, path)) => (NodeName::try_from(*name)?, path),
            // Nothing left in the path, we've found our target just validate the node actually
            None => return Ok(WalkState::found(working_directory_id)),
        };

        let perm_id = match children.get(&current_entry) {
            Some(pid) => pid,
            None => {
                return Ok(WalkState::MissingComponent {
                    working_directory_id,
                    missing_name: current_entry,
                    remaining_path,
                });
            }
        };

        let next_node_id = *inner_read
            .permanent_id_map
            .get(perm_id)
            .ok_or(OperationError::MissingPermanentId(*perm_id))?;

        let next_node = &inner_read.nodes[next_node_id];
        if !matches!(next_node.kind(), NodeKind::Directory { .. }) {
            return Ok(WalkState::NotTraversable {
                working_directory_id,
                blocking_name: current_entry,
            });
        }
        drop(inner_read);

        if depth >= MAX_PATH_DEPTH {
            return Err(OperationError::PathTooDeep);
        }

        walk_path(inner, next_node_id, remaining_path, depth + 1).await
    }
    .boxed()
}
