use std::ops::Deref;

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::AsyncWrite;
use nom::multi::count;
use nom::sequence::tuple;
use nom::{IResult, Needed};

use crate::codec::crypto::{AccessKey, AsymLockedAccessKey, KeyId, SigningKey};
use crate::codec::meta::ActorSettings;

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
    ) -> IResult<&'a [u8], Option<Self>> {
        let mut asym_parser = count(
            tuple((KeyId::parse, AsymLockedAccessKey::parse)),
            key_count as usize,
        );

        let (input, locked_keys) = match asym_parser(input) {
            Ok(res) => res,
            Err(nom::Err::Incomplete(Needed::Size(_))) => {
                let record_size = KeyId::size() + AsymLockedAccessKey::size();
                let total_size = key_count as usize * record_size;

                return Err(nom::Err::Incomplete(Needed::new(total_size - input.len())));
            }
            Err(err) => return Err(err),
        };

        let key_id = signing_key.key_id();
        let _span = tracing::debug_span!("parse_access", ?key_id).entered();

        let mut meta_key = None;
        let relevant_keys = locked_keys.iter().filter(|(kid, _)| *kid == key_id);

        for (key_id, potential_key) in relevant_keys {
            tracing::info!(candidate_key_id = ?key_id, "found_candidate");

            if let Ok(key) = potential_key.unlock(signing_key) {
                tracing::info!("successful_decrypt");
                meta_key = Some(Self::from(key));
                break;
            }
        }

        Ok((input, meta_key))
    }
}

impl std::fmt::Debug for MetaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MetaKey(*redacted*)")
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
