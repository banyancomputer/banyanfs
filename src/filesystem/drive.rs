use std::path::{Component, Path};

use elliptic_curve::rand_core::CryptoRngCore;
use slab::Slab;

use crate::codec::crypto::SigningKey;
use crate::codec::header::KeyAccessSettingsBuilder;
use crate::codec::meta::{ActorId, FilesystemId};
use crate::filesystem::nodes::NodeKind;
use crate::filesystem::operations::*;
use crate::filesystem::{DriveAccess, Node, NodeBuilder, NodeId, PermanentNodeId};

pub struct Drive {
    current_key: SigningKey,

    filesystem_id: FilesystemId,
    access: DriveAccess,

    nodes: Slab<Node>,
    root_node_id: NodeId,
}

impl Drive {
    pub fn has_realized_view_access(&self, actor_id: ActorId) -> bool {
        self.access.has_realized_view_access(actor_id)
    }

    pub fn has_write_access(&self, actor_id: ActorId) -> bool {
        self.access.has_write_access(actor_id)
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

        let node_entry = nodes.vacant_entry();
        let root_node_id = node_entry.key();

        let directory = NodeBuilder::directory(root_node_id, actor_id).build(rng);
        node_entry.insert(directory);

        Self {
            current_key,

            filesystem_id,
            access,

            nodes,
            root_node_id,
        }
    }

    pub(crate) fn root_directory(&mut self) -> Directory {
        Directory::new(self, self.root_node_id)
    }
}

pub struct Directory<'a> {
    drive: &'a mut Drive,
    node_id: NodeId,
}

impl<'a> Directory<'a> {
    //async fn ls(mut self, path: &Path) -> Result<Vec<(String, PermanentNodeId)>, OperationError> {
    //    let mut components = path.components();

    //    let mut active_path = components.next();
    //    if let Some(Component::RootDir) = active_path {
    //        self.node_id = self.drive.root_node_id;
    //        active_path = components.next();
    //    }

    //    let node_children = match self.drive.nodes[self.node_id].kind() {
    //        NodeKind::Directory { children, .. } => children,
    //        _ => return Err(OperationError::IncompatibleType("ls")),
    //    };

    //    if active_path.is_none() || active_path == Some(Component::CurDir) {
    //        let contents: Vec<_> = node_children
    //            .iter()
    //            .map(|(name, pid)| (name.clone(), pid.clone()))
    //            .collect();

    //        return Ok(contents);
    //    }
    //
    //    // todo: need to get the next node id , validate its a directory and then continue

    //    self.ls(components.as_path()).await
    //}

    pub fn new(drive: &'a mut Drive, node_id: NodeId) -> Self {
        debug_assert!(drive.nodes.contains(node_id));
        debug_assert!(matches!(
            drive.nodes[node_id].kind(),
            NodeKind::Directory { .. }
        ));

        Self { drive, node_id }
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
