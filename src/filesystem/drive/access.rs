use std::collections::HashMap;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};

use crate::codec::crypto::{AccessKey, VerifyingKey};
use crate::codec::header::KeyAccessSettings;
use crate::codec::{ActorId, ActorSettings, AsyncEncodable};

#[derive(Debug, Default)]
pub struct DriveAccess {
    actor_settings: HashMap<ActorId, ActorSettings>,
}

impl DriveAccess {
    pub fn actor_settings(&self, actor_id: ActorId) -> Option<KeyAccessSettings> {
        self.actor_settings
            .get(&actor_id)
            .map(|settings| settings.actor_settings())
    }

    pub async fn encode_escrow<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let key_count = self.actor_settings.len();
        if key_count == 0 || key_count > u8::MAX as usize {
            return Err(StdError::new(StdErrorKind::Other, "invalid number of keys"));
        }
        let key_count = key_count as u8;

        writer.write_all(&[key_count]).await?;
        written_bytes += 1;

        let mut actor_settings = self.actor_settings.values().collect::<Vec<_>>();
        actor_settings.sort_by_key(|settings| settings.verifying_key().actor_id());

        let meta_key = AccessKey::generate(rng);
        for settings in actor_settings.iter() {
            let verifying_key = settings.verifying_key();

            let locked_key = meta_key
                .lock_for(rng, &verifying_key)
                .map_err(|_| StdError::new(StdErrorKind::Other, "unable to escrow meta key"))?;

            written_bytes += locked_key.encode(writer).await?;
        }

        let fs_key = AccessKey::generate(rng);
        let data_key = AccessKey::generate(rng);
        let maintenance_key = AccessKey::generate(rng);

        let mut plaintext_buffer = Vec::new();

        for settings in actor_settings.iter() {
            let key_access_settings = settings.actor_settings();

            // first write out the settings for the specific actor
            written_bytes += key_access_settings.encode(&mut plaintext_buffer).await?;

            // then encrypt of fill in dummy data based on access
            let verifying_key = settings.verifying_key();

            if key_access_settings.has_filesystem_key() {
                // Byte indicating the key is present
                plaintext_buffer.write_all(&[0x01]).await?;

                let protected_key = fs_key.lock_for(rng, &verifying_key).map_err(|_| {
                    StdError::new(StdErrorKind::Other, "unable to escrow filesystem key")
                })?;

                tracing::info!(written_bytes, "before");
                written_bytes += protected_key.encode(&mut plaintext_buffer).await?;
                tracing::info!(written_bytes, "after");
            } else {
                plaintext_buffer.write_all(&[0x00]).await?;
                todo!("need to write out empty keys");
            }

            if key_access_settings.has_data_key() {
                // Byte indicating the key is present
                plaintext_buffer.write_all(&[0x01]).await?;

                let protected_key = data_key.lock_for(rng, &verifying_key).map_err(|_| {
                    StdError::new(StdErrorKind::Other, "unable to escrow filesystem key")
                })?;

                written_bytes += protected_key.encode(&mut plaintext_buffer).await?;
            } else {
                plaintext_buffer.write_all(&[0x00]).await?;
                todo!("need to write out empty keys");
            }

            if key_access_settings.has_maintenance_key() {
                // Byte indicating the key is present
                plaintext_buffer.write_all(&[0x01]).await?;

                let protected_key =
                    maintenance_key.lock_for(rng, &verifying_key).map_err(|_| {
                        StdError::new(StdErrorKind::Other, "unable to escrow filesystem key")
                    })?;

                written_bytes += protected_key.encode(&mut plaintext_buffer).await?;
            } else {
                plaintext_buffer.write_all(&[0x00]).await?;
                todo!("need to write out empty keys");
            }
        }

        let (nonce, tag) = meta_key
            .encrypt_buffer(rng, &[], &mut plaintext_buffer)
            .map_err(|_| {
                StdError::new(StdErrorKind::Other, "unable to encrypt escrowed key buffer")
            })?;

        written_bytes += nonce.encode(writer).await?;
        writer.write_all(plaintext_buffer.as_slice()).await?;
        written_bytes += plaintext_buffer.len();
        written_bytes += tag.encode(writer).await?;

        Ok(written_bytes)
    }

    pub fn has_read_access(&self, actor_id: ActorId) -> bool {
        let settings = match self.actor_settings.get(&actor_id) {
            Some(s) => s.actor_settings(),
            None => return false,
        };

        if settings.is_historical() {
            return false;
        }

        match settings {
            KeyAccessSettings::Public { .. } => true,
            KeyAccessSettings::Private {
                filesystem_key_present,
                ..
            } => filesystem_key_present,
        }
    }

    pub fn has_write_access(&self, actor_id: ActorId) -> bool {
        let settings = match self.actor_settings.get(&actor_id) {
            Some(s) => s.actor_settings(),
            None => return false,
        };

        if settings.is_historical() {
            return false;
        }

        match settings {
            KeyAccessSettings::Public { owner, .. } => owner,
            KeyAccessSettings::Private {
                filesystem_key_present,
                data_key_present,
                maintenance_key_present,
                ..
            } => filesystem_key_present && data_key_present && maintenance_key_present,
        }
    }

    pub fn new() -> Self {
        Self {
            actor_settings: HashMap::new(),
        }
    }

    pub fn register_actor(&mut self, key: VerifyingKey, settings: KeyAccessSettings) {
        let actor_id = key.actor_id();
        let actor_settings = ActorSettings::new(key, settings);

        self.actor_settings.insert(actor_id, actor_settings);
    }
}
