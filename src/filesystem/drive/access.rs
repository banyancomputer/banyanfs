use std::collections::HashMap;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;

use crate::codec::crypto::{
    AuthenticationTag, KeyId, Nonce, PermissionKeys, SigningKey, VerifyingKey,
};
use crate::codec::header::KeyAccessSettings;
use crate::codec::meta::VectorClock;
use crate::codec::{ActorId, ActorSettings, AsyncEncodable, ParserResult};
use crate::filesystem::drive::MetaKey;

#[derive(Debug, Default)]
pub struct DriveAccess {
    actor_settings: HashMap<ActorId, ActorSettings>,
    permission_keys: Option<PermissionKeys>,
}

impl DriveAccess {
    pub fn actor_settings(&self, actor_id: ActorId) -> Option<KeyAccessSettings> {
        self.actor_settings
            .get(&actor_id)
            .map(|settings| settings.actor_settings())
    }

    pub fn permission_keys(&self) -> Option<&PermissionKeys> {
        self.permission_keys.as_ref()
    }

    // todo: should use the filesystem ID as authenticated data with all the components
    pub fn recover_permissions<'a>(
        input: &'a [u8],
        key_count: u8,
        meta_key: &MetaKey,
        signing_key: &SigningKey,
    ) -> ParserResult<'a, Self> {
        let payload_size = key_count as usize * Self::size();

        let (input, nonce) = Nonce::parse(input)?;
        let (input, crypt_slice) = take(payload_size)(input)?;
        let (input, tag) = AuthenticationTag::parse(input)?;

        let mut crypt_buffer = crypt_slice.to_vec();
        if let Err(err) = meta_key.decrypt_buffer(nonce, &mut crypt_buffer, &[], tag) {
            tracing::error!("failed to decrypt permission buffer: {err}");
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        let mut actor_settings = HashMap::new();
        let mut permission_keys = None;
        let mut buffer_input = crypt_buffer.as_slice();

        for _ in 0..key_count {
            let (buf_inp, key_id) = KeyId::parse(buffer_input).map_err(|_| {
                nom::Err::Failure(nom::error::make_error(input, nom::error::ErrorKind::Verify))
            })?;
            buffer_input = buf_inp;

            let (buf_inp, settings) = ActorSettings::parse_private(buffer_input).map_err(|_| {
                nom::Err::Failure(nom::error::make_error(input, nom::error::ErrorKind::Verify))
            })?;
            buffer_input = buf_inp;

            let verifying_key = settings.verifying_key();
            let actor_id = verifying_key.actor_id();
            actor_settings.insert(actor_id, settings);

            if key_id == verifying_key.key_id() {
                match PermissionKeys::parse(buffer_input, signing_key) {
                    Ok((buf_inp, keys)) => {
                        permission_keys = Some(keys);
                        buffer_input = buf_inp;
                        continue;
                    }
                    Err(err) => tracing::error!("failed to access permission keys: {err}"),
                };
            }

            buffer_input = buffer_input.split_at(PermissionKeys::size()).1;
        }

        if permission_keys.is_none() {
            tracing::warn!("no matching permission keys found for provided key");
        }

        Ok((
            input,
            Self {
                actor_settings,
                permission_keys,
            },
        ))
    }

    pub async fn encode_permissions<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        meta_key: &MetaKey,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let permission_keys = PermissionKeys::generate(rng);
        let mut plaintext_buffer = Vec::new();

        for settings in self.sorted_actor_settings().iter() {
            let verifying_key = settings.verifying_key();
            let key_id = verifying_key.key_id();
            key_id.encode(&mut plaintext_buffer).await?;

            settings.encode(&mut plaintext_buffer).await?;

            let key_settings = settings.actor_settings();

            // and the protection keys based on their access
            permission_keys
                .encode_for(rng, &mut plaintext_buffer, &key_settings, &verifying_key)
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

        tracing::debug!(written_bytes, "encode_permissions::complete");

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
            permission_keys: None,
        }
    }

    pub fn register_actor(&mut self, key: VerifyingKey, settings: KeyAccessSettings) {
        let actor_id = key.actor_id();
        let actor_settings = ActorSettings::new(key, settings);

        self.actor_settings.insert(actor_id, actor_settings);
    }

    pub fn sorted_actor_settings(&self) -> Vec<&ActorSettings> {
        let mut actors: Vec<(&ActorId, &ActorSettings)> = self.actor_settings.iter().collect();
        actors.sort_by(|(aid, _), (bid, _)| aid.key_id().cmp(&bid.key_id()));
        actors.into_iter().map(|(_, settings)| settings).collect()
    }

    pub const fn size() -> usize {
        KeyId::size()
            + VerifyingKey::size()
            + VectorClock::size()
            + KeyAccessSettings::size()
            + PermissionKeys::size()
    }
}
