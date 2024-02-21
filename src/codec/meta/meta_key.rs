use std::io::{Error as StdError, ErrorKind as StdErrorKind};
use std::ops::Deref;

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::AsyncWrite;
use nom::multi::count;
use nom::sequence::tuple;
use nom::Needed;

use crate::codec::crypto::{AccessKey, AsymLockedAccessKey, KeyId, SigningKey};
use crate::codec::header::KeyCount;
use crate::codec::{ActorSettings, AsyncEncodable, ParserResult};

pub struct MetaKey(AccessKey);

impl MetaKey {
    pub(crate) async fn encode_escrow<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        mut actor_settings: Vec<&ActorSettings>,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let key_count = KeyCount::try_from(actor_settings.len())?;
        written_bytes += key_count.encode(writer).await?;

        actor_settings.sort_by_key(|settings| settings.verifying_key().actor_id());

        for settings in actor_settings.iter() {
            let verifying_key = settings.verifying_key();
            let key_id = verifying_key.key_id();

            let locked_key = self
                .lock_for(rng, &verifying_key)
                .map_err(|_| StdError::new(StdErrorKind::Other, "unable to escrow meta key"))?;

            written_bytes += key_id.encode(writer).await?;
            written_bytes += locked_key.encode(writer).await?;
        }

        Ok(written_bytes)
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        MetaKey(AccessKey::generate(rng))
    }

    pub fn parse_escrow<'a>(
        input: &'a [u8],
        key_count: u8,
        signing_key: &SigningKey,
    ) -> ParserResult<'a, Option<Self>> {
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

        let signing_key_id = signing_key.key_id();
        let mut meta_key = None;

        for (key_id, potential_key) in locked_keys.iter().filter(|(kid, _)| *kid == signing_key_id)
        {
            tracing::trace!(candidate_key_id = ?key_id, "found_candidate");

            if let Ok(key) = potential_key.unlock(signing_key) {
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
