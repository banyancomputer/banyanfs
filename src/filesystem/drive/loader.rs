use std::sync::Arc;

use async_std::sync::RwLock;
use futures::{AsyncRead, AsyncReadExt};
use nom::number::streaming::le_u64;
use tracing::{debug, trace};

use crate::codec::crypto::{AuthenticationTag, EncryptedBuffer, Nonce, SigningKey};
use crate::codec::header::{ContentOptions, IdentityHeader, KeyCount, PublicSettings};
use crate::codec::meta::{FilesystemId, JournalCheckpoint, MetaKey};
use crate::codec::parser::{
    ParserResult, ParserStateMachine, ProgressType, SegmentStreamer, StateError, StateResult,
};
use crate::filesystem::{Drive, DriveAccess, InnerDrive};

pub struct DriveLoader<'a> {
    signing_key: &'a SigningKey,
    state: DriveLoaderState,

    filesystem_id: Option<FilesystemId>,
    public_settings: Option<PublicSettings>,
    drive_access: Option<DriveAccess>,
}

impl<'a> DriveLoader<'a> {
    pub fn new(signing_key: &'a SigningKey) -> Self {
        Self {
            signing_key,
            state: DriveLoaderState::IdentityHeader,

            filesystem_id: None,
            public_settings: None,
            drive_access: None,
        }
    }

    pub async fn from_reader<R: AsyncRead + AsyncReadExt + Unpin>(
        self,
        mut reader: R,
    ) -> Result<Drive, DriveLoaderError> {
        let mut streamer = SegmentStreamer::new(self);

        loop {
            let mut buffer = vec![0; 1024];
            let bytes_read = reader.read(&mut buffer).await?;

            // Handle EOF
            if bytes_read == 0 {
                return Err(DriveLoaderError::UnexpectedStreamEnd);
            }

            let (data, _) = buffer.split_at(bytes_read);
            let read_bytes = bytes::Bytes::from(data.to_owned());
            streamer.add_chunk(&read_bytes);

            if let Some(segment_res) = streamer.next().await {
                let (hash, drive) = segment_res?;
                let root_cid = drive.root_cid().await;
                debug!(drive_hash = ?hash, drive_root_cid = ?root_cid, "loaded drive");
                return Ok(drive);
            };
        }
    }
}

impl ParserStateMachine<Drive> for DriveLoader<'_> {
    type Error = DriveLoaderError;

    fn parse(&mut self, buffer: &[u8]) -> StateResult<Drive, Self::Error> {
        match &self.state {
            DriveLoaderState::IdentityHeader => {
                let (input, id_header) = IdentityHeader::parse_with_magic(buffer)?;
                let bytes_read = buffer.len() - input.len();

                trace!(bytes_read, ?id_header, "drive_loader::identity_header");

                self.state = DriveLoaderState::FilesystemId;

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::FilesystemId => {
                let (input, filesystem_id) = FilesystemId::parse(buffer)?;
                let bytes_read = buffer.len() - input.len();

                trace!(bytes_read, ?filesystem_id, "drive_loader::filesystem_id");

                self.filesystem_id = Some(filesystem_id);
                self.state = DriveLoaderState::PublicSettings;

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::PublicSettings => {
                let (input, public_settings) = PublicSettings::parse(buffer)?;
                let bytes_read = buffer.len() - input.len();

                trace!(
                    bytes_read,
                    ?public_settings,
                    "drive_loader::public_settings"
                );

                self.public_settings = Some(public_settings);
                self.state = DriveLoaderState::KeyCount;

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::KeyCount => {
                let (input, key_count) = KeyCount::parse(buffer)?;
                let bytes_read = buffer.len() - input.len();

                trace!(bytes_read, ?key_count, "drive_loader::key_count");

                if self
                    .public_settings
                    .as_ref()
                    .expect("to have been set")
                    .private()
                {
                    self.state = DriveLoaderState::EscrowedAccessKeys(key_count);
                } else {
                    unimplemented!("public filesystems not yet available");
                }

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::EscrowedAccessKeys(key_count) => {
                // todo(sstelfox): switch to EncryptedBuffer
                let (input, meta_key) =
                    MetaKey::parse_escrow(buffer, **key_count, self.signing_key)?;

                let bytes_read = buffer.len() - input.len();
                trace!(bytes_read, ?key_count, "drive_loader::escrowed_access_keys");

                let meta_key = match meta_key {
                    Some(mk) => mk,
                    None => return Err(DriveLoaderError::AccessUnavailable),
                };

                trace!("drive_loader::escrowed_access_keys::unlocked");
                self.state = DriveLoaderState::EncryptedHeader(*key_count, meta_key);

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::EncryptedHeader(key_count, meta_key) => {
                let payload_size = (**key_count as usize * DriveAccess::size())
                    + ContentOptions::size()
                    + JournalCheckpoint::size();

                let (input, header_buffer) =
                    EncryptedBuffer::parse_and_decrypt(buffer, payload_size, &[], meta_key)?;
                let encrypted_size = buffer.len() - input.len();
                trace!(
                    encrypted_size,
                    payload_size,
                    "drive_loader::encrypted_header"
                );

                let mut header = header_buffer.as_ref();

                let (remaining, drive_access) =
                    DriveAccess::parse(header, **key_count, self.signing_key)?;
                let bytes_read = header.len() - remaining.len();
                (_, header) = header.split_at(bytes_read);
                trace!(bytes_read, "drive_loader::encrypted_header::drive_access");

                let (remaining, content_options) = ContentOptions::parse(header)?;
                let bytes_read = header.len() - remaining.len();
                (_, header) = header.split_at(bytes_read);
                trace!(
                    bytes_read,
                    "drive_loader::encrypted_header::content_options"
                );

                let (remaining, journal_start) = JournalCheckpoint::parse(header)?;
                let bytes_read = header.len() - remaining.len();
                trace!(
                    bytes_read,
                    "drive_loader::encrypted_header::journal_checkpoint"
                );

                debug_assert!(remaining.is_empty());

                self.drive_access = Some(drive_access);
                self.state = DriveLoaderState::PrivateContent(content_options, journal_start);

                let bytes_read = buffer.len() - input.len();
                trace!(bytes_read, "drive_loader::encrypted_header::complete");

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::PrivateContent(content_options, journal_start) => {
                if content_options.include_filesystem() {
                    let (input, encrypted_size) = content_length(buffer)?;

                    let encrypted_size = encrypted_size as usize;
                    let payload_size = encrypted_size - (Nonce::size() + AuthenticationTag::size());

                    let filesystem_key = self
                        .drive_access
                        .as_ref()
                        .and_then(|a| a.permission_keys())
                        .and_then(|pk| pk.filesystem.as_ref())
                        .ok_or(DriveLoaderError::KeyNotAvailable("filesystem key missing"))?;

                    let data_key = self
                        .drive_access
                        .as_ref()
                        .and_then(|a| a.permission_keys())
                        .and_then(|pk| pk.data.as_ref());

                    // todo(sstelfox): we ideally want to stream this data and selectively parse
                    // things, but that has impacts on the encryption which would need to be managed
                    // carefully. Since this only covers the realized view of the filesystem (the
                    // metadata) and no file content this shouldn't grow very large.

                    // todo(sstelfox): authenticated data should include filesystem ID, and length
                    // bytes

                    let (input, fs_buffer) = EncryptedBuffer::parse_and_decrypt(
                        input,
                        payload_size,
                        &[],
                        filesystem_key,
                    )?;

                    trace!(
                        encrypted_size,
                        payload_size,
                        "drive_loader::private_content::decrypt_successful"
                    );
                    let drive_access = self.drive_access.clone().expect("to have been set");

                    let (remaining, inner_drive) = InnerDrive::parse(
                        &fs_buffer,
                        drive_access,
                        journal_start.clone(),
                        data_key,
                    )?;
                    debug_assert!(remaining.is_empty());

                    let drive = Drive {
                        current_key: Arc::new(self.signing_key.clone()),
                        filesystem_id: self.filesystem_id.expect("to have been set"),
                        private: true,
                        inner: Arc::new(RwLock::new(inner_drive)),
                    };

                    // todo handle journal entries

                    let bytes_read = buffer.len() - input.len();
                    trace!(bytes_read, "drive_loader::encrypted_payload::complete");

                    return Ok(ProgressType::Ready(bytes_read, drive));
                }

                // todo handle data segments

                unimplemented!("further content");
            }
        }
    }
}

fn content_length(input: &[u8]) -> ParserResult<u64> {
    le_u64(input)
}

#[derive(Debug, thiserror::Error)]
pub enum DriveLoaderError {
    #[error("the provided signing key does not have access to this encrypted filesystem")]
    AccessUnavailable,

    #[error("additional data needed to continue parsing")]
    Incomplete(Option<usize>),

    #[error("failed to decrypt internal data with associated key: {0}")]
    InternalKeyError(&'static str),

    #[error("an I/O error occurred: {0}")]
    IoError(#[from] std::io::Error),

    #[error("key expected to be available was missing when it was needed: {0}")]
    KeyNotAvailable(&'static str),

    #[error("failed to parse drive data: {0}")]
    ParserFailure(String),

    #[error("unexpected end of stream")]
    UnexpectedStreamEnd,
}

impl StateError for DriveLoaderError {
    fn needed_data(&self) -> Option<usize> {
        match self {
            DriveLoaderError::Incomplete(n) => *n,
            _ => None,
        }
    }

    fn needs_more_data(&self) -> bool {
        matches!(self, DriveLoaderError::Incomplete(_))
    }
}

impl<E: std::fmt::Debug> From<nom::Err<E>> for DriveLoaderError {
    fn from(err: nom::Err<E>) -> Self {
        match err {
            nom::Err::Incomplete(nom::Needed::Size(n)) => {
                DriveLoaderError::Incomplete(Some(n.get()))
            }
            nom::Err::Incomplete(_) => DriveLoaderError::Incomplete(None),
            err => {
                let err_msg = format!("parse verification detected failure: {:?}", err);
                DriveLoaderError::ParserFailure(err_msg)
            }
        }
    }
}

#[derive(Debug)]
enum DriveLoaderState {
    IdentityHeader,
    FilesystemId,
    PublicSettings,

    KeyCount,

    EscrowedAccessKeys(KeyCount),
    EncryptedHeader(KeyCount, MetaKey),
    PrivateContent(ContentOptions, JournalCheckpoint),
    //PublicPermissions(KeyCount),
    //PublicContent,

    //Signature,
    //ErrorCorrection,
}
