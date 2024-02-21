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
pub(crate) use operations::OperationError;

pub(crate) use walk_state::WalkState;

use std::collections::{HashMap, HashSet};
use std::io::{Error as StdError, ErrorKind as StdErrorKind};
use std::ops::Deref;
use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use slab::Slab;
use tracing::trace;

use crate::codec::crypto::*;
use crate::codec::header::*;
use crate::codec::*;
use crate::filesystem::nodes::{Node, NodeBuilder, NodeBuilderError, NodeId};

pub struct Drive {
    current_key: Arc<SigningKey>,
    filesystem_id: FilesystemId,

    private: bool,

    // todo: need to switch to a mutex, can't have state being modified during an encoding session
    // and the cooperative multitasking model of async/await means we can't guarantee that some
    // other task isn't going to tweak it
    inner: Arc<RwLock<InnerDrive>>,
}

impl Drive {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        content_options: ContentOptions,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        if self.private {
            self.encode_private(rng, content_options, writer).await
        } else {
            unimplemented!("public encoding not implemented")
        }
    }

    async fn encode_private<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        content_options: ContentOptions,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += IdentityHeader::encode(&IdentityHeader, writer).await?;
        written_bytes += self.filesystem_id.encode(writer).await?;

        // Don't support ECC yet
        written_bytes += PublicSettings::new(false, true).encode(writer).await?;

        let meta_key = MetaKey::generate(rng);
        let inner_read = self.inner.read().await;

        let key_list = inner_read.access.sorted_actor_settings();

        written_bytes += meta_key.encode_escrow(rng, writer, key_list).await?;

        let mut header_buffer = EncryptedBuffer::default();

        inner_read.access.encode(rng, &mut *header_buffer).await?;
        content_options.encode(&mut *header_buffer).await?;
        inner_read.journal_start.encode(&mut *header_buffer).await?;

        // todo: include filesystem ID and encoded length bytes as AD
        written_bytes += header_buffer
            .encrypt_and_encode(writer, rng, &[], meta_key.deref())
            .await?;

        if content_options.include_filesystem() {
            let mut fs_buffer = EncryptedBuffer::default();

            let filesystem_key = inner_read
                .access
                .permission_keys()
                .and_then(|pk| pk.filesystem.as_ref())
                .ok_or(StdError::new(StdErrorKind::Other, "no filesystem key"))?;

            written_bytes += inner_read.encode(&mut *fs_buffer).await?;

            // todo: use filesystem ID and encoded length bytes as AD
            let buffer_length = fs_buffer.encrypted_len() as u64;
            let length_bytes = buffer_length.to_le_bytes();
            writer.write_all(&length_bytes).await?;
            written_bytes += length_bytes.len();

            written_bytes += fs_buffer
                .encrypt_and_encode(writer, rng, &[], filesystem_key)
                .await?;
        }

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
        trace!(?actor_id, ?filesystem_id, "drive::initializing_private");

        let kas = KeyAccessSettingsBuilder::private()
            .set_owner()
            .set_protected()
            .with_all_access()
            .build();

        let mut access = DriveAccess::init_private(rng, actor_id);
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

        let journal_start = JournalCheckpoint::initialize();

        let drive = Self {
            current_key,
            filesystem_id,
            private: true,

            inner: Arc::new(RwLock::new(InnerDrive {
                access,

                journal_start,

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

pub(crate) struct InnerDrive {
    access: DriveAccess,

    journal_start: JournalCheckpoint,

    pub(crate) nodes: Slab<Node>,
    pub(crate) root_node_id: NodeId,
    pub(crate) permanent_id_map: HashMap<PermanentId, NodeId>,
}

impl InnerDrive {
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let all_perms = self
            .access
            .permission_keys()
            .ok_or(StdError::new(StdErrorKind::Other, "no permission keys"))?;

        // todo(sstelfox): there is a complex use case here that needs to be handled. Someone with
        // access to the filesystem and maintenance key, but without the data key can make changes
        // as long as they preserve the data key they loaded the filesystem with. Ideally data
        // wrapping keys would be rotated everytime there was a full save but that won't work for
        // now. Both these cases can be iteratively added on later to the library.

        let data_key = all_perms
            .data
            .as_ref()
            .ok_or(StdError::new(StdErrorKind::Other, "no data key"))?;

        // Walk the nodes starting from the root, encoding them one at a time, we want to make sure
        // we only encode things once and do so in a consistent order to ensure our content is
        // reproducible. This will silently discard any disconnected leaf nodes. Loops are
        // tolerated.

        let mut seen_ids = HashSet::new();
        let mut outstanding_ids = vec![self.root_node_id];
        //let mut data_ids = Vec::new();

        let mut node_encoding_buffer = Vec::new();

        while let Some(node_id) = outstanding_ids.pop() {
            let node = self.nodes.get(node_id).ok_or_else(|| {
                StdError::new(StdErrorKind::Other, "node ID missing from internal nodes")
            })?;

            // Deduplicate nodes as we go through them
            let permanent_id = node.permanent_id();
            if seen_ids.contains(&permanent_id) {
                continue;
            }
            seen_ids.insert(permanent_id);

            permanent_id.encode(&mut node_encoding_buffer).await?;
            node.owner_id().encode(&mut node_encoding_buffer).await?;

            let created_at_bytes = node.created_at().to_le_bytes();
            node_encoding_buffer.write_all(&created_at_bytes).await?;

            let modified_at_bytes = node.modified_at().to_le_bytes();
            node_encoding_buffer.write_all(&modified_at_bytes).await?;

            node.name().encode(&mut node_encoding_buffer).await?;

            writer.write_all(&node_encoding_buffer).await?;
            written_bytes += node_encoding_buffer.len();
        }

        Ok(written_bytes)
    }

    pub fn parse(_input: &[u8]) -> ParserResult<Self> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error("failed to build a drive entry: {0}")]
    NodeBuilderError(#[from] NodeBuilderError),

    #[error("operation on the drive failed due to an error: {0}")]
    OperationError(#[from] OperationError),
}
