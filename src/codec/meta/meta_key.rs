use std::ops::Deref;

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::AsyncWrite;

use crate::codec::crypto::{AccessKey, AsymLockedAccessKey, SigningKey};
use crate::codec::meta::ActorSettings;

#[derive(Debug)]
pub struct MetaKey(AccessKey);

impl MetaKey {
    async fn encode_access<W: AsyncWrite + Unpin + Send>(
        &self,
        _writer: &mut W,
        _actor_settings: Vec<&ActorSettings>,
    ) -> std::io::Result<usize> {
        todo!()
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        MetaKey(AccessKey::generate(rng))
    }

    pub fn parse_access<'a>(
        input: &'a [u8],
        key_count: u8,
        signing_key: &SigningKey,
    ) -> nom::IResult<&'a [u8], Option<Self>> {
        let (input, locked_keys) = AsymLockedAccessKey::parse_many(input, key_count)?;

        let key_id = signing_key.key_id();
        let _span = tracing::debug_span!("parse_access", signing_key_id = ?key_id).entered();

        let mut meta_key = None;
        let relevant_keys = locked_keys.iter().filter(|k| k.key_id == key_id);

        for potential_key in relevant_keys {
            tracing::trace!(potential_key = ?potential_key.key_id(), "candidate_check");

            if let Ok(key) = potential_key.unlock(signing_key) {
                tracing::info!("successful_decrypt");
                meta_key = Some(Self::from(key));
                break;
            }
        }

        Ok((input, meta_key))
    }
}

impl Deref for MetaKey {
    type Target = AccessKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<AccessKey> for MetaKey {
    fn from(key: AccessKey) -> Self {
        MetaKey(key)
    }
}
