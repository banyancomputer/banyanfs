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

        let escrow_key = AccessKey::generate(rng);

        for settings in self.actor_settings.values() {
            let verifying_key = settings.verifying_key();

            let locked_key = escrow_key
                .lock_for(rng, &verifying_key)
                .map_err(|_| StdError::new(StdErrorKind::Other, "unable to escrow key"))?;

            written_bytes += locked_key.encode(writer).await?;
        }

        Ok(written_bytes)
    }

    pub fn has_realized_view_access(&self, actor_id: ActorId) -> bool {
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
                realized_key_present,
                ..
            } => realized_key_present,
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
                realized_key_present,
                data_key_present,
                journal_key_present,
                ..
            } => realized_key_present && data_key_present && journal_key_present,
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
