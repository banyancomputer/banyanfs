mod access;
mod directory_handle;
mod file_handle;
mod loader;
mod operations;
mod walk_state;

pub use access::DriveAccess;
pub use directory_handle::DirectoryHandle;
pub use file_handle::FileHandle;
pub use loader::{DriveLoader, DriveLoaderError};
pub use operations::OperationError;

pub(crate) use walk_state::WalkState;

use std::collections::HashMap;
use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::AsyncWrite;
use slab::Slab;
use tracing::debug;

use crate::codec::crypto::*;
use crate::codec::header::*;
use crate::codec::*;
use crate::filesystem::nodes::{Node, NodeBuilder, NodeBuilderError, NodeId};

pub struct Drive {
    current_key: Arc<SigningKey>,
    filesystem_id: FilesystemId,
    // todo: need to switch to a mutex, can't have state being modified during an encoding session
    // and the cooperative multitasking model of async/await means we can't guarantee that some
    // other task isn't going to tweak it
    inner: Arc<RwLock<InnerDrive>>,
}

pub(crate) struct InnerDrive {
    access: DriveAccess,
    pub(crate) nodes: Slab<Node>,
    pub(crate) root_node_id: NodeId,
    pub(crate) permanent_id_map: HashMap<PermanentId, NodeId>,
}

impl Drive {
    pub async fn encode_private<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += IdentityHeader::encode(&IdentityHeader, writer).await?;
        written_bytes += self.filesystem_id.encode(writer).await?;

        // Don't support ECC yet
        written_bytes += PublicSettings::new(false, true).encode(writer).await?;

        let inner_read = self.inner.read().await;
        written_bytes += inner_read.access.encode_escrow(rng, writer).await?;

        Ok(written_bytes)
    }

    pub async fn has_read_access(&self, actor_id: ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access.has_read_access(actor_id)
    }

    pub async fn has_write_access(&self, actor_id: ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access.has_write_access(actor_id)
    }

    pub fn id(&self) -> FilesystemId {
        self.filesystem_id
    }

    pub fn initialize_private(
        rng: &mut impl CryptoRngCore,
        current_key: SigningKey,
    ) -> Result<Self, DriveError> {
        let verifying_key = current_key.verifying_key();
        let actor_id = verifying_key.actor_id();

        let filesystem_id = FilesystemId::generate(rng);
        debug!(?actor_id, ?filesystem_id, "drive::initializing_private");

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

        let directory = NodeBuilder::root()
            .with_id(root_node_id)
            .with_owner(actor_id)
            .build(rng)?;

        permanent_id_map.insert(directory.permanent_id(), root_node_id);
        node_entry.insert(directory);

        let current_key = Arc::new(current_key);

        let drive = Self {
            current_key,
            filesystem_id,

            inner: Arc::new(RwLock::new(InnerDrive {
                access,
                nodes,
                root_node_id,
                permanent_id_map,
            })),
        };

        Ok(drive)
    }

    pub async fn root(&self) -> DirectoryHandle {
        let inner_read = self.inner.read().await;
        let root_node_id = inner_read.root_node_id;
        drop(inner_read);

        DirectoryHandle::new(self.current_key.clone(), root_node_id, self.inner.clone()).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error("failed to build a drive entry: {0}")]
    NodeBuilderError(#[from] NodeBuilderError),

    #[error("operation on the drive failed due to an error: {0}")]
    OperationError(#[from] OperationError),
}
