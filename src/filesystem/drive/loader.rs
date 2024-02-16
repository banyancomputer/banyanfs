use futures::{AsyncRead, AsyncReadExt};

use crate::codec::crypto::{AccessKey, AsymLockedAccessKey, SigningKey};
use crate::codec::header::{IdentityHeader, KeyCount, PublicSettings};
use crate::codec::meta::{ContentContext, FilesystemId};
use crate::codec::parser::{
    ParserStateMachine, ProgressType, SegmentStreamer, StateError, StateResult,
};
use crate::filesystem::Drive;

pub struct DriveLoader<'a> {
    signing_key: &'a SigningKey,
    state: DriveLoaderState,

    filesystem_id: Option<FilesystemId>,
    public_settings: Option<PublicSettings>,
}

impl<'a> DriveLoader<'a> {
    pub fn new(signing_key: &'a SigningKey) -> Self {
        Self {
            signing_key,
            state: DriveLoaderState::IdentityHeader,

            filesystem_id: None,
            public_settings: None,
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
                tracing::info!("loaded drive with blake3 hash of {{{hash:02x?}}}");
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

                tracing::debug!(bytes_read, ?id_header, "drive_loader::identity_header");

                self.state = DriveLoaderState::FilesystemId;

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::FilesystemId => {
                let (input, filesystem_id) = FilesystemId::parse(buffer)?;
                let bytes_read = buffer.len() - input.len();

                tracing::debug!(bytes_read, ?filesystem_id, "drive_loader::filesystem_id");

                self.filesystem_id = Some(filesystem_id);
                self.state = DriveLoaderState::PublicSettings;

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::PublicSettings => {
                let (input, public_settings) = PublicSettings::parse(buffer)?;
                let bytes_read = buffer.len() - input.len();

                tracing::debug!(
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

                tracing::debug!(bytes_read, ?key_count, "drive_loader::key_count");

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
                let (input, locked_keys) = AsymLockedAccessKey::parse_many(buffer, **key_count)?;
                let bytes_read = buffer.len() - input.len();

                let key_id = self.signing_key.key_id();
                tracing::debug!(bytes_read, signing_key_id = ?key_id, ?locked_keys, "drive_loader::escrowed_access_keys");

                let mut key_access_key = None;
                let relevant_keys = locked_keys.iter().filter(|k| k.key_id == key_id);

                for potential_key in relevant_keys {
                    tracing::debug!(matching_key = ?potential_key, "drive_loader::escrowed_access_keys");

                    if let Ok(key) = potential_key.unlock(self.signing_key) {
                        key_access_key = Some(key);
                        break;
                    }
                }

                let key_access_key = match key_access_key {
                    Some(ak) => ak,
                    None => return Err(DriveLoaderError::AccessUnavailable),
                };
                tracing::debug!(unlocked_key = ?key_access_key, "drive_loader::escrowed_access_keys");

                self.state = DriveLoaderState::EncryptedPermissions(*key_count, key_access_key);

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::EncryptedPermissions(_key_count, _access_key) => {
                todo!()
            }
            remaining => {
                unimplemented!("parsing for state {remaining:?}");
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveLoaderError {
    #[error("additional data needed to continue parsing")]
    Incomplete(Option<usize>),

    #[error("an I/O error occurred: {0}")]
    IoError(#[from] std::io::Error),

    #[error("the provided signing key does not have access to this encrypted filesystem")]
    AccessUnavailable,

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

impl<E> From<nom::Err<E>> for DriveLoaderError {
    fn from(err: nom::Err<E>) -> Self {
        match err {
            nom::Err::Incomplete(nom::Needed::Size(n)) => {
                DriveLoaderError::Incomplete(Some(n.get()))
            }
            nom::Err::Incomplete(_) => DriveLoaderError::Incomplete(None),
            _ => DriveLoaderError::ParserFailure(
                "failed to parse data, hard error types live here".to_string(),
            ),
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
    EncryptedPermissions(KeyCount, AccessKey),
    PrivateContentPayload(ContentContext),

    PublicAccessKeys(KeyCount),
    PublicContentPayload(ContentContext),

    Signature,
    ErrorCorrection,
}
