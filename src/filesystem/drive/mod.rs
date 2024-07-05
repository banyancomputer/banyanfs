mod access;
mod directory_entry;
mod directory_handle;
mod inner;
mod loader;
mod operations;
mod walk_state;

pub use access::{DriveAccess, DriveAccessError};
pub use directory_entry::DirectoryEntry;
pub use directory_handle::DirectoryHandle;
pub use loader::{DriveLoader, DriveLoaderError};
pub use operations::OperationError;

pub(crate) use inner::InnerDrive;
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

use crate::filesystem::nodes::{Node, NodeBuilderError, NodeName};

/// The core entry point of the library, a `Drive` is the means through which the BanyanFS
/// filesystem's public or private data is accessed. Initial creation of a new drive requires a
/// [`SigningKey`] to be provided to [`Drive::initialize_private`]. It is up to the consumer of the
/// library to store and load the private key, and the metadata of the
/// filesystem produce via [`Drive::encode`]. File data itself is not stored in the drive, but uses
/// the provided [`crate::stores::DataStore`] to store the data blocks.
///
/// Most of the operations on the drive itself are done through [`DirectoryHandle`] instances and
/// [`DirectoryEntry`] instances, which are used to navigate the filesystem and access the both
/// file and associated data directly. The [`Drive::root`] method allows grabbing a handle on the
/// root directory of the filesystem to begin performing operations on them.
///
/// # Examples
///
/// ```rust
/// use std::sync::Arc;
/// use banyanfs::prelude::*;
/// # let mut rng = rand::thread_rng();
/// let signing_key = Arc::new(SigningKey::generate(&mut rng));
/// let new_drive = Drive::initialize_private(&mut rng, signing_key);
/// ```
#[derive(Clone)]
pub struct Drive {
    filesystem_id: FilesystemId,
    private: bool,

    current_key: Arc<SigningKey>,
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
            .vector_clock()
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
                .filesystem_key()
                .ok_or(StdError::new(StdErrorKind::Other, "no filesystem key"))?;

            written_bytes += inner_read.encode(&mut *fs_buffer).await?;

            // todo(sstelfox): use filesystem ID and encoded length bytes as AD, but this is a
            // breaking change...

            let buffer_length = fs_buffer.encrypted_len() as u64;
            let length_bytes = buffer_length.to_le_bytes();
            writer.write_all(&length_bytes).await?;
            written_bytes += length_bytes.len();

            written_bytes += fs_buffer
                .encrypt_and_encode(rng, writer, &[], filesystem_key)
                .await?;
        }

        Ok(written_bytes)
    }

    pub async fn has_maintenance_access(&self, actor_id: &ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access().has_maintenance_access(actor_id)
    }

    pub async fn has_read_access(&self, actor_id: &ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access().has_read_access(actor_id)
    }

    pub async fn has_write_access(&self, actor_id: &ActorId) -> bool {
        let inner = self.inner.read().await;
        inner.access().has_write_access(actor_id)
    }

    pub fn id(&self) -> FilesystemId {
        self.filesystem_id
    }

    /// Create a new encrypted drive with the provided [`SigningKey`]. This will generate a random
    /// fileystem ID.
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
        let vector_clock_actor = VectorClockActor::initialize(actor_id);

        trace!(?actor_id, ?filesystem_id, "drive::initializing_private");

        let access = DriveAccess::initialize(rng, verifying_key, vector_clock_actor.as_snapshot())?;
        let inner = InnerDrive::initialize(rng, actor_id, access, vector_clock_actor)?;

        let drive = Self {
            current_key,
            filesystem_id,
            private: true,
            inner: Arc::new(RwLock::new(inner)),
        };

        Ok(drive)
    }

    /// Registers a new key as an actor with the provided AccessMask. Will produce an error if used
    /// to attempt to change the permissions of an existing key.
    pub async fn authorize_key(
        &self,
        rng: &mut impl CryptoRngCore,
        key: VerifyingKey,
        access_mask: AccessMask,
    ) -> Result<(), DriveAccessError> {
        let mut inner_write = self.inner.write().await;
        let vector_clock_snapshot = inner_write.vector_clock().actor();
        inner_write
            .access_mut()
            .register_actor(rng, key, access_mask, vector_clock_snapshot)?;
        Ok(())
    }

    /// Marks the key with the matching actor id as historical. Requires that the corresponding key
    /// not be protected. Requires that the current key be an owner.
    pub async fn remove_key(
        &self,
        current_key: &SigningKey,
        removal_id: &ActorId,
    ) -> Result<(), DriveAccessError> {
        let mut inner_write = self.inner.write().await;
        inner_write
            .access_mut()
            .remove_actor(current_key, removal_id)?;
        Ok(())
    }
    /// If the caller knows the [`PermanentId`] of a directory, they can retrieve a handle on it
    /// directly. Generally users will traverse the filesystem themselves to get this information,
    /// but that can be a costly operation. This allows the external cacheing of permanent IDs to
    /// quickly jump to a specific location.
    ///
    /// Will produce an error if the provided permanent ID is not a traversable type.
    pub async fn directory_by_id(
        &self,
        permanent_id: &PermanentId,
    ) -> Result<DirectoryHandle, OperationError> {
        let inner_read = self.inner.read().await;

        let node = inner_read.by_perm_id(permanent_id)?;
        if !node.supports_children() {
            return Err(OperationError::NotTraversable);
        }

        let current_key = self.current_key.clone();
        let handle = DirectoryHandle::new(current_key, node.id(), self.inner.clone()).await;

        Ok(handle)
    }

    /// Retrieve a handle on a specific entry in the filesystem by its permanent ID. This requires
    /// the caller to already know the [`PermanentId`] they are looking for. Generally, users and
    /// software operate by performing a walk of the filesystem themselves to read a particular
    /// entry. This method is primarily useful if an external cache is used for these kinds of
    /// mappings.
    pub async fn entry_by_id(
        &self,
        permanent_id: &PermanentId,
    ) -> Result<DirectoryEntry, OperationError> {
        let inner_read = self.inner.read().await;

        let node = inner_read.by_perm_id(permanent_id)?;
        let entry = DirectoryEntry::try_from(node)?;

        Ok(entry)
    }

    pub async fn for_each_node<F, R>(&self, operation: F) -> Result<Vec<R>, OperationError>
    where
        F: Fn(&Node) -> Result<Option<R>, OperationError> + Send + Sync,
    {
        let inner_read = self.inner.read().await;

        let mut responses = Vec::new();
        for node in inner_read.node_iter() {
            if let Some(resp) = operation(node)? {
                responses.push(resp);
            }
        }

        Ok(responses)
    }

    pub fn rekey_data_references(_rng: &mut impl CryptoRngCore) -> Result<(), DriveError> {
        todo!("not needed yet, but keeping as a placeholder")
    }

    /// Retrieve a handle on the root directory of the filesystem. This is the starting point for
    /// most initial traversal and is the foundation of the filesystem structure. Attempting to use
    /// relative paths "above" this location will result in an Error.
    pub async fn root(&self) -> Result<DirectoryHandle, OperationError> {
        let inner_read = self.inner.read().await;
        let root_perm_id = inner_read.root_pid();

        self.directory_by_id(&root_perm_id).await
    }

    pub async fn root_cid(&self) -> Result<Cid, DriveError> {
        let inner_read = self.inner.read().await;

        let root_node = inner_read.root_node()?;
        let root_cid = root_node.cid().await?;

        Ok(root_cid)
    }

    pub async fn full_path_from_root(
        &self,
        target: &PermanentId,
    ) -> Result<Vec<String>, OperationError> {
        let inner_read = &self.inner.read().await;
        let mut target_node = inner_read
            .by_perm_id(target)
            .map_err(|_| OperationError::MissingPermanentId(*target))?;

        let target_node_name = match target_node.name() {
            NodeName::Root => return Ok(Vec::new()),
            NodeName::Named(name) => name.to_string(),
        };

        let mut path = vec![target_node_name];
        while let Some(parent_id) = target_node.parent_id() {
            let parent_node = inner_read.by_perm_id(&parent_id)?;

            match parent_node.name() {
                NodeName::Root => break,
                NodeName::Named(name) => path.push(name.to_string()),
            }
            target_node = parent_node
        }
        path.reverse();

        Ok(path)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error("a failure occurred attempting to modify drive access controls: {0}")]
    AccessChangeFailure(#[from] DriveAccessError),

    #[error("failed to build a drive entry: {0}")]
    NodeBuilderError(#[from] NodeBuilderError),

    #[error("operation on the drive failed due to an error: {0}")]
    OperationError(#[from] OperationError),
}
