mod access;
mod data_store;
mod directory_entry;
mod directory_handle;
mod file_handle;
mod inner;
mod loader;
mod operations;
mod walk_state;

pub use access::DriveAccess;
pub use data_store::{DataStore, DataStoreError};
pub use directory_entry::DirectoryEntry;
pub use directory_handle::DirectoryHandle;
pub use file_handle::FileHandle;
pub use loader::{DriveLoader, DriveLoaderError};

pub(crate) use inner::InnerDrive;
pub(crate) use operations::OperationError;
pub(crate) use walk_state::WalkState;

use std::io::{Error as StdError, ErrorKind as StdErrorKind};
use std::ops::Deref;
use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use tracing::trace;

use crate::codec::crypto::*;
use crate::codec::header::*;
use crate::codec::*;
use crate::filesystem::nodes::NodeBuilderError;

#[derive(Clone)]
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

        let key_list = inner_read.access().sorted_actor_settings();
        written_bytes += meta_key.encode_escrow(rng, writer, key_list).await?;

        let mut header_buffer = EncryptedBuffer::default();

        let mut inner_header_size = inner_read.access().encode(rng, &mut *header_buffer).await?;
        inner_header_size += content_options.encode(&mut *header_buffer).await?;
        inner_header_size += inner_read
            .journal_start()
            .encode(&mut *header_buffer)
            .await?;

        // todo: include filesystem ID and encoded length bytes as AD
        let hdr_len = header_buffer
            .encrypt_and_encode(rng, writer, &[], meta_key.deref())
            .await?;
        written_bytes += hdr_len;
        tracing::trace!(payload_size = ?inner_header_size, encrypted_size = ?hdr_len, "drive::encode_private::header");

        if content_options.include_filesystem() {
            let mut fs_buffer = EncryptedBuffer::default();

            let filesystem_key = inner_read
                .access()
                .permission_keys()
                .and_then(|pk| pk.filesystem.as_ref())
                .ok_or(StdError::new(StdErrorKind::Other, "no filesystem key"))?
                .clone();

            written_bytes += inner_read.encode(&mut *fs_buffer).await?;

            // todo: use filesystem ID and encoded length bytes as AD
            let buffer_length = fs_buffer.encrypted_len() as u64;
            let length_bytes = buffer_length.to_le_bytes();
            writer.write_all(&length_bytes).await?;
            written_bytes += length_bytes.len();

            written_bytes += fs_buffer
                .encrypt_and_encode(rng, writer, &[], &filesystem_key)
                .await?;
        }

        Ok(written_bytes)
    }

    pub async fn has_read_access(&self, actor_id: ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access().has_read_access(actor_id)
    }

    pub async fn has_write_access(&self, actor_id: ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access().has_write_access(actor_id)
    }

    pub fn id(&self) -> FilesystemId {
        self.filesystem_id
    }

    pub fn initialize_private(
        rng: &mut impl CryptoRngCore,
        current_key: Arc<SigningKey>,
    ) -> Result<Self, DriveError> {
        let filesystem_id = FilesystemId::generate(rng);
        Self::initialize_private_with_id(rng, current_key, filesystem_id)
    }

    pub fn initialize_private_with_id(
        rng: &mut impl CryptoRngCore,
        current_key: Arc<SigningKey>,
        filesystem_id: FilesystemId,
    ) -> Result<Self, DriveError> {
        let verifying_key = current_key.verifying_key();
        let actor_id = verifying_key.actor_id();

        trace!(?actor_id, ?filesystem_id, "drive::initializing_private");

        let kas = KeyAccessSettingsBuilder::private()
            .set_owner()
            .set_protected()
            .with_all_access()
            .build();

        let mut access = DriveAccess::init_private(rng, actor_id);
        access.register_actor(verifying_key, kas);

        let inner = InnerDrive::initialize(rng, actor_id, access.clone())?;

        let drive = Self {
            current_key,
            filesystem_id,
            private: true,
            inner: Arc::new(RwLock::new(inner)),
        };

        Ok(drive)
    }

    pub async fn root(&self) -> DirectoryHandle {
        let inner_read = self.inner.read().await;
        let root_node_id = inner_read.root_node_id();
        DirectoryHandle::new(self.current_key.clone(), root_node_id, self.inner.clone()).await
    }

    pub async fn root_cid(&self) -> Result<Cid, DriveError> {
        let inner_read = self.inner.read().await;

        let root_node = inner_read.root_node()?;
        let root_cid = root_node.cid().await?;

        Ok(root_cid)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error("failed to build a drive entry: {0}")]
    NodeBuilderError(#[from] NodeBuilderError),

    #[error("operation on the drive failed due to an error: {0}")]
    OperationError(#[from] OperationError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[ignore]
    async fn test_drive_lifecycle() {
        let mut _rng = crate::utils::crypto_rng();
        todo!()
    }
}
