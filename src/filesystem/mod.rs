mod content_reference;
mod file_content;
mod nodes;
mod private_encoding_context;

pub use content_reference::ContentReference;
pub use file_content::FileContent;
pub use nodes::*;
pub(crate) use private_encoding_context::PrivateEncodingContext;

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite};
use nom::number::streaming::le_u64;
use nom::IResult;

use crate::codec::content_payload::{
    ContentOptions, ContentPayload, KeyAccessSettings, PermissionControl,
};
use crate::codec::crypto::{AccessKey, AsymLockedAccessKey, SigningKey, VerifyingKey};
use crate::codec::header::{IdentityHeader, KeyCount, PublicSettings};
use crate::codec::{
    ActorId, AsyncEncodable, Cid, FilesystemId, ParserStateMachine, ProgressType, SegmentStreamer,
    StateError, StateResult,
};

pub(crate) type KeyMap = HashMap<ActorId, (VerifyingKey, KeyAccessSettings)>;

pub struct Drive {
    filesystem_id: FilesystemId,
    keys: KeyMap,
    root: Directory,
}

impl Drive {
    pub fn check_accessibility(&self, key: &VerifyingKey) -> bool {
        match self.keys.get(&key.actor_id()) {
            Some((_, kas)) => match kas {
                KeyAccessSettings::Public { historical, .. } => !historical,
                KeyAccessSettings::Private {
                    historical,
                    realized_key_present,
                    ..
                } => !historical && *realized_key_present,
            },
            None => false,
        }
    }

    pub async fn encode_private<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        _signing_key: &SigningKey,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += IdentityHeader::encode(&IdentityHeader, writer).await?;
        written_bytes += self.filesystem_id.encode(writer).await?;

        // Don't support ECC yet
        written_bytes += PublicSettings::new(false, true).encode(writer).await?;

        let encoding_context = PrivateEncodingContext::new(
            rng,
            self.keys.clone(),
            (0, 0),
            (Cid::from([0u8; 32]), Cid::from([0u8; 32])),
        );

        let content_payload = ContentPayload::Private;
        written_bytes += content_payload
            .encode_private(rng, &encoding_context, writer)
            .await?;

        Ok(written_bytes)
    }

    pub fn id(&self) -> FilesystemId {
        self.filesystem_id
    }

    pub fn initialize_private(rng: &mut impl CryptoRngCore, signing_key: &SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        let actor_id = signing_key.actor_id();

        let kas = KeyAccessSettings::Private {
            protected: true,
            owner: true,
            historical: false,

            realized_key_present: true,
            data_key_present: true,
            journal_key_present: true,
            maintenance_key_present: true,
        };

        let mut keys = HashMap::new();
        keys.insert(actor_id, (verifying_key, kas));

        Self {
            filesystem_id: FilesystemId::generate(rng),
            keys,
            root: Directory::new(rng, actor_id),
        }
    }

    pub fn is_writable(&self, key: &SigningKey) -> bool {
        match self.keys.get(&key.actor_id()) {
            Some((_, kas)) => match kas {
                KeyAccessSettings::Public { historical, .. } => !historical,
                KeyAccessSettings::Private {
                    historical,
                    data_key_present,
                    journal_key_present,
                    realized_key_present,
                    ..
                } => {
                    !historical
                        && *realized_key_present
                        && *data_key_present
                        && *journal_key_present
                }
            },
            None => false,
        }
    }
}

impl Deref for Drive {
    type Target = Directory;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

impl DerefMut for Drive {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.root
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error("failed to parse drive data, is this a banyanfs file?")]
    HeaderReadFailure,
}

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

    pub async fn load_from_reader<R: AsyncRead + AsyncReadExt + Unpin>(
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

#[derive(Debug)]
pub(crate) struct VectorClock(u64);

impl VectorClock {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = le_u64(input)?;
        Ok((input, Self(value)))
    }
}

#[derive(Debug)]
pub struct JournalCheckpoint {
    merkle_root_cid: Cid,
    vector: VectorClock,
}

impl JournalCheckpoint {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, merkle_root_cid) = Cid::parse(input)?;
        let (input, vector) = VectorClock::parse(input)?;

        let journal_checkpoint = JournalCheckpoint {
            merkle_root_cid,
            vector,
        };

        Ok((input, journal_checkpoint))
    }
}

#[derive(Debug)]
enum DriveLoaderState {
    IdentityHeader,
    FilesystemId,
    PublicSettings,

    KeyCount,

    EscrowedAccessKeys(KeyCount),
    EncryptedContentPayloadEntry(AccessKey),

    PrivateContentPayload(JournalCheckpoint, ContentOptions, Vec<PermissionControl>),

    PublicContentPayload(KeyCount),
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
                    self.state = DriveLoaderState::PublicContentPayload(key_count);
                }

                Ok(ProgressType::Advance(bytes_read))
            }
            DriveLoaderState::EscrowedAccessKeys(key_count) => {
                let (input, locked_keys) = AsymLockedAccessKey::parse_many(buffer, **key_count)?;
                let bytes_read = buffer.len() - input.len();

                let key_id = self.signing_key.key_id();
                tracing::debug!(bytes_read, signing_key_id = ?key_id, ?locked_keys, "drive_loader::escrowed_access_keys");

                let relevant_keys = locked_keys.iter().filter(|k| k.key_id == key_id);
                tracing::debug!(?relevant_keys, "drive_loader::escrowed_access_keys");

                let mut key_access_key = None;
                for potential_key in relevant_keys {
                    if let Ok(key) = potential_key.unlock(self.signing_key) {
                        key_access_key = Some(key);
                        break;
                    }
                }

                tracing::debug!(unlocked_key = ?key_access_key, "drive_loader::escrowed_access_keys");
                let key_access_key = match key_access_key {
                    Some(ak) => ak,
                    None => return Err(DriveLoaderError::AccessUnavailable),
                };

                self.state = DriveLoaderState::EncryptedContentPayloadEntry(key_access_key);

                Ok(ProgressType::Advance(bytes_read))
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
