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
use futures::AsyncWrite;

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
