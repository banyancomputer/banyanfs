use std::ops::Deref;

use async_trait::async_trait;
use ecdsa::signature::rand_core::CryptoRngCore;
use elliptic_curve::pkcs8::EncodePublicKey;
use elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::error::ErrorKind;
use nom::{Err, IResult};
use p384::ecdh::EphemeralSecret;
use p384::{NistP384, PublicKey};

use crate::codec::crypto::{AccessKey, Fingerprint, KeyId};
use crate::codec::ActorId;
use crate::codec::AsyncEncodable;

const KEY_SIZE: usize = 49;

#[derive(Clone)]
pub struct VerifyingKey {
    inner: ecdsa::VerifyingKey<NistP384>,
}

impl VerifyingKey {
    pub fn actor_id(&self) -> ActorId {
        ActorId::from(self.fingerprint())
    }

    pub(crate) fn ephemeral_dh_exchange(&self, rng: &mut impl CryptoRngCore) -> (Self, AccessKey) {
        let eph_secret: EphemeralSecret = EphemeralSecret::random(rng);

        let pub_key = Self {
            inner: eph_secret.public_key().into(),
        };

        let shared_secret = eph_secret.diffie_hellman(&self.inner.into());
        let secret_expansion = shared_secret.extract::<sha2::Sha384>(None);

        let mut secret_bytes = [0u8; 32];
        if secret_expansion.expand(&[], &mut secret_bytes).is_err() {
            unreachable!("secret_bytes will always have the correct length");
        }

        (pub_key, AccessKey::from(secret_bytes))
    }

    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::from(self)
    }

    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, VerifyingKeyError> {
        todo!()
    }

    #[cfg(feature = "pem")]
    pub fn from_spki(pem: &str) -> Result<Self, VerifyingKeyError> {
        use elliptic_curve::pkcs8::DecodePublicKey;

        let p384_key =
            PublicKey::from_public_key_pem(pem).map_err(VerifyingKeyError::InvalidSpki)?;
        let inner = ecdsa::VerifyingKey::from(p384_key);

        Ok(Self { inner })
    }

    pub fn key_id(&self) -> KeyId {
        self.fingerprint().key_id()
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (remaining, slice) = take(KEY_SIZE)(input)?;

        let mut bytes = [0u8; KEY_SIZE];
        bytes.copy_from_slice(slice);

        let key = match ecdsa::VerifyingKey::from_sec1_bytes(&bytes) {
            Ok(key) => key,
            Err(err) => {
                tracing::error!("failed to decode ECDSA key: {err}");
                return Err(Err::Failure(nom::error::Error::new(input, ErrorKind::Fail)));
            }
        };

        Ok((remaining, Self { inner: key }))
    }

    pub const fn size() -> usize {
        KEY_SIZE
    }

    pub fn to_bytes(&self) -> [u8; KEY_SIZE] {
        let compressed_public_key = self.inner.to_encoded_point(true);

        let mut public_key = [0u8; KEY_SIZE];
        public_key.copy_from_slice(compressed_public_key.as_bytes());

        public_key
    }

    pub fn to_spki(&self) -> Result<String, VerifyingKeyError> {
        let public_key: PublicKey = self.inner.into();

        let spki = public_key
            .to_public_key_pem(elliptic_curve::pkcs8::LineEnding::LF)
            .map_err(VerifyingKeyError::SpkiEncodingFailed)?;

        Ok(spki)
    }
}

#[async_trait]
impl AsyncEncodable for VerifyingKey {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let key_bytes = self.to_bytes();
        writer.write_all(&key_bytes).await?;
        Ok(key_bytes.len())
    }
}

impl std::fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{VerifyingKey({:?})}}", self.key_id())
    }
}

impl Deref for VerifyingKey {
    type Target = ecdsa::VerifyingKey<NistP384>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<ecdsa::VerifyingKey<NistP384>> for VerifyingKey {
    fn from(inner: ecdsa::VerifyingKey<NistP384>) -> Self {
        Self { inner }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyingKeyError {
    #[error("failed to decode verifying key from compressed bytes: {0}")]
    InvalidByteEncoding(elliptic_curve::Error),

    #[error("failed to load SPKI fomatted verifying key: {0}")]
    InvalidSpki(elliptic_curve::pkcs8::spki::Error),

    #[error("failed to load SPKI encoded verifying key: {0}")]
    SpkiDecodingFailed(elliptic_curve::Error),

    #[error("failed to encoded public key as SPKI: {0}")]
    SpkiEncodingFailed(elliptic_curve::pkcs8::spki::Error),
}
