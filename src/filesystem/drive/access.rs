use std::collections::HashMap;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};

use crate::codec::crypto::{AccessKey, PermissionKeys, VerifyingKey};
use crate::codec::header::{KeyAccessSettings, KeyCount};
use crate::codec::{ActorId, ActorSettings, AsyncEncodable};
use crate::filesystem::drive::MetaKey;

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

    pub async fn parse_escrow<W: AsyncWrite + Unpin + Send>(
        &mut self,
        _reader: &mut W,
        _key_count: u8,
        _meta_key: &AccessKey,
    ) -> std::io::Result<usize> {
        todo!()
    }

    pub async fn encode_escrow<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        meta_key: &MetaKey,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let key_count = KeyCount::try_from(self.actor_settings.len())?;
        written_bytes += key_count.encode(writer).await?;

        // todo(sstelfox): begin cut section, this part needs to be separated out into its own decoder/encoder
        let mut actor_settings = self.actor_settings.values().collect::<Vec<_>>();
        actor_settings.sort_by_key(|settings| settings.verifying_key().actor_id());

        for settings in actor_settings.iter() {
            let verifying_key = settings.verifying_key();

            let locked_key = meta_key
                .lock_for(rng, &verifying_key)
                .map_err(|_| StdError::new(StdErrorKind::Other, "unable to escrow meta key"))?;

            written_bytes += locked_key.encode(writer).await?;
        }
        // end cut section

        let permission_keys = PermissionKeys::generate(rng);
        let mut plaintext_buffer = Vec::new();

        for settings in actor_settings.iter() {
            let verifying_key = settings.verifying_key();

            // write key ID out
            let key_id = verifying_key.key_id();
            key_id.encode(&mut plaintext_buffer).await?;

            // write pubkey out
            verifying_key.encode(&mut plaintext_buffer).await?;

            // the actor's current clock
            let actor_clock = settings.vector_clock();
            actor_clock.encode(&mut plaintext_buffer).await?;

            // the actor key settings
            let actor_settings = settings.actor_settings();
            actor_settings.encode(&mut plaintext_buffer).await?;

            // and the protection keys based on their access
            permission_keys
                .encode_for(rng, &mut plaintext_buffer, &actor_settings, &verifying_key)
                .await?;
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
