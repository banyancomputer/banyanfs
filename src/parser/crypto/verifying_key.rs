use std::ops::Deref;

use ecdsa::signature::rand_core::CryptoRngCore;
use elliptic_curve::sec1::ToEncodedPoint;
use nom::bytes::streaming::take;
use nom::combinator::all_consuming;
use nom::error::{ErrorKind, ParseError};
use nom::{Err, IResult};
use p384::ecdh::EphemeralSecret;
use p384::NistP384;
use rand::Rng;

use crate::parser::crypto::KeyId;

const KEY_SIZE: usize = 49;

#[derive(Clone)]
pub(crate) struct VerifyingKey {
    inner_key: ecdsa::VerifyingKey<NistP384>,
}

impl VerifyingKey {
    pub(crate) fn ephemeral_dh_exchange(&self, rng: &mut impl CryptoRngCore) -> (Self, [u8; 32]) {
        let eph_secret: EphemeralSecret = EphemeralSecret::random(rng);

        let pub_key = Self {
            inner_key: eph_secret.public_key().into(),
        };

        let shared_secret = eph_secret.diffie_hellman(&self.inner_key.into());
        let secret_expansion = shared_secret.extract::<sha2::Sha384>(None);

        let mut secret_bytes = [0u8; 32];
        if secret_expansion.expand(&[], &mut secret_bytes).is_err() {
            unreachable!("secret_bytes will always have the correct length");
        }

        (pub_key, secret_bytes)
    }

    pub(crate) fn key_id(&self) -> KeyId {
        let public_key_bytes = self.inner_key.to_encoded_point(true);
        let public_key_hash = blake3::hash(public_key_bytes.as_bytes());

        let mut key_id = [0u8; 2];
        key_id.copy_from_slice(public_key_hash.as_bytes());

        KeyId::from(u16::from_le_bytes(key_id))
    }

    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (remaining, slice) = take(KEY_SIZE)(input)?;

        let mut bytes = [0u8; KEY_SIZE];
        bytes.copy_from_slice(slice);

        let key = match ecdsa::VerifyingKey::from_sec1_bytes(&bytes) {
            Ok(key) => key,
            Err(err) => return Err(Err::Failure(nom::error::Error::new(input, ErrorKind::Fail))),
        };

        Ok((remaining, Self { inner_key: key }))
    }

    pub(crate) const fn size() -> usize {
        KEY_SIZE
    }

    pub(crate) fn to_bytes(&self) -> [u8; KEY_SIZE] {
        let compressed_pubkey = self.inner_key.to_encoded_point(true);

        let mut public_key = [0u8; KEY_SIZE];
        public_key.copy_from_slice(compressed_pubkey.as_bytes());

        public_key
    }
}

impl Deref for VerifyingKey {
    type Target = ecdsa::VerifyingKey<NistP384>;

    fn deref(&self) -> &Self::Target {
        &self.inner_key
    }
}

impl From<ecdsa::VerifyingKey<NistP384>> for VerifyingKey {
    fn from(inner_key: ecdsa::VerifyingKey<NistP384>) -> Self {
        Self { inner_key }
    }
}
