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
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, TryStream, TryStreamExt};

use crate::codec::content_payload::{ContentPayload, KeyAccessSettings};
use crate::codec::crypto::{SigningKey, VerifyingKey};
use crate::codec::header::{IdentityHeader, PublicSettings};
use crate::codec::{ActorId, AsyncEncodable, Cid, FilesystemId};

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

    pub fn load_with_key(input: &[u8], signing_key: &SigningKey) -> Result<Self, DriveError> {
        let (input, _) =
            IdentityHeader::parse_with_magic(input).map_err(|_| DriveError::HeaderReadFailure)?;
        let (input, filesystem_id) =
            FilesystemId::parse(input).map_err(|_| DriveError::HeaderReadFailure)?;
        let (input, public_settings) =
            PublicSettings::parse(input).map_err(|_| DriveError::HeaderReadFailure)?;

        todo!()
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

use crate::codec::{ParserStateMachine, SegmentStreamer, StateError, StateResult};
use bytes::Bytes;

pub struct DriverLoader<'a> {
    signing_key: &'a SigningKey,
    state: DriverLoaderState,
}

impl<'a> DriverLoader<'a> {
    pub fn new(signing_key: &'a SigningKey) -> Self {
        Self {
            signing_key,
            state: DriverLoaderState::IdentityHeader,
        }
    }

    pub async fn load_from_reader<R: AsyncRead + Unpin>(
        self,
        mut reader: R,
    ) -> Result<Drive, DriveLoaderError> {
        let mut buffer: bytes::BytesMut;
        let mut streamer = SegmentStreamer::new(self);

        loop {
            buffer = bytes::BytesMut::new();
            let bytes_read = reader.read(&mut buffer).await?;

            // Handle EOF
            if bytes_read == 0 {
                return Err(DriveLoaderError::UnexpectedStreamEnd);
            }

            let bytes_chunk = buffer.freeze();
            streamer.add_chunk(&bytes_chunk);

            if let Some(segment_res) = streamer.next().await {
                let (hash, drive) = segment_res?;
                tracing::info!("loaded drive with blake3 hash of {{{hash:02x?}}}");
                return Ok(drive);
            };
        }
    }

    pub async fn load_from_stream<S>(self, mut stream: S) -> Result<Drive, DriveLoaderError>
    where
        S: TryStream<Ok = Bytes, Error = std::io::Error> + Unpin,
    {
        let mut streamer = SegmentStreamer::new(self);

        while let Some(chunk) = stream.try_next().await? {
            streamer.add_chunk(&chunk);

            if let Some(segment_res) = streamer.next().await {
                let (hash, drive) = segment_res?;
                tracing::info!("loaded drive with blake3 hash of {{{hash:02x?}}}");
                return Ok(drive);
            };
        }

        Err(DriveLoaderError::UnexpectedStreamEnd)
    }
}

enum DriverLoaderState {
    IdentityHeader,
    FilesystemId,
    PublicSettings,
}

impl ParserStateMachine<Drive> for DriverLoader<'_> {
    type Error = DriveLoaderError;

    fn parse(&mut self, buffer: &[u8]) -> StateResult<Drive, Self::Error> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveLoaderError {
    #[error("additional data needed to continue parsing")]
    Incomplete(Option<usize>),

    #[error("an I/O error occurred: {0}")]
    IoError(#[from] std::io::Error),

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
