use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use tracing::{debug, instrument, trace, Instrument, Level};

use crate::codec::*;
use crate::filesystem::nodes::NodeKind;
use crate::filesystem::{Node, NodeId};

use crate::codec::crypto::SigningKey;
use crate::filesystem::drive::{InnerDrive, OperationError, WalkState};

pub struct DirectoryHandle {
    current_key: Arc<SigningKey>,
    cwd_id: NodeId,
    inner: Arc<RwLock<InnerDrive>>,
}

impl DirectoryHandle {
    // todo: these operations should really be using the permanent ids, but is that worth the extra
    // level of indirection? As long as we remain consistent it should be fine.
    #[instrument(level = Level::TRACE, skip(self))]
    async fn walk_path<'b>(&self, path: &'b [&'b str]) -> Result<WalkState<'b>, OperationError> {
        trace!("directory::walk_directory");

        if path.is_empty() {
            return Ok(WalkState::found(self.cwd_id));
        }

        todo!()

        //loop {
        //    let (current_entry, remaining_path) = match path.split_first() {
        //        Some(r) => r,
        //        None => {
        //            return Err(OperationError::UnexpectedEmptyPath);
        //        }
        //    };

        //    match *current_entry {
        //        // This path references where we currently are, do nothing and let the next loop
        //        // advance our state
        //        "." => (),
        //        // Zip up the directory level
        //        ".." => match self.inner.read().in_current_span().await.nodes[cwd_id].parent_id() {
        //            Some(parent_id) => {
        //                cwd_id = parent_id;
        //            }
        //            None => return Err(OperationError::InvalidParentDir),
        //        },
        //        cur_name => {
        //            if cur_name.is_empty() {
        //                return Err(OperationError::NameIsEmpty);
        //            }

        //            if cur_name.len() > 255 {
        //                return Err(OperationError::PathComponentTooLong);
        //            }

        //            let inner_read = self.inner.read().await;
        //            let node_children = match inner_read.nodes[cwd_id].kind() {
        //                NodeKind::Directory { children, .. } => children,
        //                _ => {
        //                    return Err(OperationError::IncompatibleType(
        //                        "current node is not a directory",
        //                    ))
        //                }
        //            };

        //            let name = NodeName::from(cur_name);
        //            let perm_id = match node_children.get(&name) {
        //                Some(perm_id) => perm_id,
        //                None => return Ok(WalkState::MissingComponent(cwd_id, path)),
        //            };

        //            let next_cwd_id = inner_read
        //                .permanent_id_map
        //                .get(perm_id)
        //                .ok_or(OperationError::MissingPermanentId(*perm_id))?;

        //            if remaining_path.is_empty() {
        //                return Ok(WalkState::Found(*next_cwd_id));
        //            }

        //            // The path goes deeper, make sure the next node is a directory
        //            let next_node = &inner_read.nodes[*next_cwd_id];
        //            if !matches!(next_node.kind(), NodeKind::Directory { .. }) {
        //                return Ok(WalkState::NotTraversable {
        //                    working_directory: cwd_id,
        //                    missing_name: cur_name,
        //                    remaining_path,
        //                });
        //            }

        //            // Go deeper on our next loop, importantly though an empty path from this point on
        //            // means we've successfully reached our goal.
        //            cwd_id = *next_cwd_id;
        //            path = remaining_path;
        //        }
        //    }
        //}
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn cd(&self, path: &[&str]) -> Result<DirectoryHandle, OperationError> {
        debug!(cwd_id = self.cwd_id, "directory::cd");

        let target_directory_id = if path.is_empty() {
            self.cwd_id
        } else {
            //match self.walk_directory(self.cwd_id, path).await {
            //    Ok(WalkState::FoundNode(tdi_id, _)) => tdi_id,
            //    _ => return Err(OperationError::NotADirectory),
            //}
            todo!()
        };

        let directory = DirectoryHandle {
            current_key: self.current_key.clone(),
            cwd_id: target_directory_id,
            inner: self.inner.clone(),
        };

        Ok(directory)
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn ls(self, path: &[&str]) -> Result<Vec<(String, PermanentId)>, OperationError> {
        debug!(cwd_id = self.cwd_id, "directory::ls");

        todo!()

        //let (target_dir_id, entry) = match self.walk_directory(self.cwd_id, path).await? {
        //    WalkState::FoundNode(tdi_id) => tdi_id,
        //    // If the last item is a file we can display it, this matches the behavior in linux
        //    // like shells
        //    WalkState::NotTraversable(tdi, blocked_path) if blocked_path.len() == 1 => {
        //        (tdi, blocked_path[0])
        //    }
        //    WalkState::NotTraversable(_, _) => return Err(OperationError::NotADirectory),
        //    WalkState::Missing(_, _) => return Err(OperationError::PathNotFound),
        //};

        //let inner_read = self.inner.read().await;
        //let target_dir = &inner_read.nodes[target_dir_id];

        //let node_children = match target_dir.kind() {
        //    NodeKind::Directory { children, .. } => children,
        //    NodeKind::File { .. } => {
        //        return Ok(vec![(entry.to_string(), target_dir.permanent_id())]);
        //    }
        //};

        //let children = node_children.iter().map(|(k, v)| (k.clone(), *v)).collect();

        //Ok(children)
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
        if parent_node.is_directory() {
            return Err(OperationError::ParentMustBeDirectory);
        }

        let mut inner_write = self.inner.write().in_current_span().await;
        let node_entry = inner_write.nodes.vacant_entry();
        let node_id = node_entry.key();

        let owner_id = self.current_key.actor_id();
        let node = build_node(rng, parent_id, node_id, owner_id)
            .in_current_span()
            .await?;

        let _name = node.name();
        let permanent_id = node.permanent_id();

        node_entry.insert(node);
        inner_write.permanent_id_map.insert(permanent_id, node_id);

        debug!(?node_id, ?permanent_id, "directory::insert_node::inserted");

        // todo: associate this node to its parent.

        Ok((node_id, permanent_id))
    }

    #[instrument(skip(self, rng))]
    pub async fn mkdir(
        &mut self,
        rng: &mut impl CryptoRngCore,
        mut path: &[&str],
        recursive: bool,
    ) -> Result<(), OperationError> {
        let mut cwd_id = self.cwd_id;
        debug!(?cwd_id, "drive::mkdir");

        if path.is_empty() {
            return Err(OperationError::UnexpectedEmptyPath);
        }

        todo!()

        //loop {
        //    match self.walk_directory(cwd_id, path).await? {
        //        // node already exists, we'll double check its a folder and consider it a success
        //        // if it is
        //        WalkState::Found(nid, entry_name) => {
        //            match self.inner.read().in_current_span().await.nodes[nid].kind() {
        //                NodeKind::Directory { .. } => {
        //                    debug!(directory = entry_name, "drive::mkdir::already_exists");
        //                    return Ok(());
        //                }
        //                _ => return Err(OperationError::NotADirectory),
        //            }
        //        }
        //        WalkState::NotTraversable(_, _) => return Err(OperationError::NotADirectory),
        //        WalkState::Missing(current_dir_id, missing_path) if !missing_path.is_empty() => {
        //            // we should always have
        //            let (missing_name, remaining_path) = match missing_path.split_first() {
        //                Some(res) => res,
        //                _ => unreachable!("protected by branch guard"),
        //            };

        //            tracing::debug!(cwd_id = ?current_dir_id, name = ?missing_name, "drive::mkdir::missing_directory");

        //            // If there is more to the path and we are not in recursive mode, we should
        //            // report the error
        //            if !(recursive || remaining_path.is_empty()) {
        //                tracing::debug!("drive::mkdir::not_recursive");
        //                return Err(OperationError::PathNotFound);
        //            }

        //            let (node_id, permanent_id) = self
        //                .insert_node(
        //                    rng,
        //                    current_dir_id,
        //                    |rng, name, actor_id, node_id| async move {
        //                        let dir = NodeBuilder::directory(name)
        //                            .with_node_id(node_id)
        //                            .with_owner(actor_id)
        //                            .with_parent(current_dir_id)
        //                            .build(rng)?;

        //                        Ok(dir)
        //                    },
        //                )
        //                .await?;

        //            let mut inner_write = self.inner.write().in_current_span().await;
        //            let node_children = match inner_write.nodes[current_dir_id].kind_mut() {
        //                NodeKind::Directory { children, .. } => children,
        //                _ => return Err(OperationError::BadSearch("not in a directory?")),
        //            };

        //            node_children.insert(missing_name.to_string(), permanent_id);
        //            tracing::debug!(
        //                cwd_id = ?current_dir_id,
        //                name = missing_name,
        //                pid = ?permanent_id,
        //                "drive::mkdir::created"
        //            );

        //            if remaining_path.is_empty() {
        //                // Nothing left to do, let's bail out
        //                return Ok(());
        //            }

        //            cwd_id = node_id;
        //            path = remaining_path;
        //        }
        //        // This shouldn't happen as this branch should be considered "Found" posibly an
        //        // empty path error, either way we shouldn't be here
        //        WalkState::Missing(_, _) => {
        //            return Err(OperationError::BadSearch("unexpected lack of directory"));
        //        }
        //    };
        //}
    }
}
