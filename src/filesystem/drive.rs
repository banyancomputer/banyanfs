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
    current_key: SigningKey,
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
        _rng: &mut impl CryptoRngCore,
        _path: &Path,
        _recursive: bool,
    ) -> Result<Directory, OperationError> {
        todo!()
    }

    pub(crate) async fn root_directory(&mut self) -> Directory {
        let inner_read = self.inner.read().await;
        let root_node_id = inner_read.root_node_id;
        drop(inner_read);

        Directory::new(&self.current_key, root_node_id, self.inner.clone()).await
    }
}

pub struct Directory<'a> {
    current_key: &'a SigningKey,
    cwd_id: NodeId,
    inner: Arc<RwLock<InnerDrive>>,
}

impl<'a> Directory<'a> {
    async fn walk_directory(&self, path: &Path) -> Result<NodeId, OperationError> {
        let mut components = path.components();
        let mut cwd_id = self.cwd_id;

        loop {
            let mut active_path = match components.next() {
                Some(path) => path,
                None => return Err(OperationError::UnexpectedEmptyPath),
            };

            match active_path {
                Component::RootDir => {
                    cwd_id = self.inner.read().await.root_node_id;
                }
                _ => (),
            }

            let node_children = match self.inner.read().await.nodes[cwd_id].kind() {
                NodeKind::Directory { children, .. } => children,
                _ => return Err(OperationError::IncompatibleType("ls")),
            };

            todo!()
        }
    }

    async fn ls(mut self, path: &Path) -> Result<Vec<(String, PermanentId)>, OperationError> {
        let mut components = path.components();
        let mut cwd_id = self.cwd_id;

        let inner_read = self.inner.read().await;

        loop {
            let mut active_path = components.next();
            if let Some(Component::RootDir) = active_path {
                cwd_id = inner_read.root_node_id;
                active_path = components.next();
            }

            let node_children = match inner_read.nodes[cwd_id].kind() {
                NodeKind::Directory { children, .. } => children,
                _ => return Err(OperationError::IncompatibleType("ls")),
            };

            match active_path {
                Some(Component::CurDir) => (),
                None => {
                    let contents: Vec<_> = node_children
                        .iter()
                        .map(|(name, pid)| (name.clone(), *pid))
                        .collect();

                    return Ok(contents);
                }
                _ => (),
            }

            todo!()
        }
    }

    pub async fn new(
        current_key: &'a SigningKey,
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

    pub fn mkdir(
        &mut self,
        _rng: &mut impl CryptoRngCore,
        _path: &Path,
        _recursive: bool,
    ) -> Result<Directory, OperationError> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error("failed to parse drive data, is this a banyanfs file?")]
    HeaderReadFailure,
}
