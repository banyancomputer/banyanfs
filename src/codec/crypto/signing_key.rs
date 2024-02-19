use ecdsa::signature::rand_core::CryptoRngCore;
use ecdsa::signature::RandomizedDigestSigner;
use p384::{NistP384, SecretKey};
use zeroize::Zeroizing;

use crate::codec::crypto::{AccessKey, Fingerprint, KeyId, Signature, VerifyingKey};
use crate::codec::ActorId;

const KEY_SIZE: usize = 48;

#[derive(Clone)]
pub struct SigningKey {
    inner: ecdsa::SigningKey<NistP384>,
}

impl SigningKey {
    pub fn actor_id(&self) -> ActorId {
        self.verifying_key().actor_id()
    }

    pub(crate) fn dh_exchange(&self, other_pubkey: &VerifyingKey) -> AccessKey {
        let shared_secret = elliptic_curve::ecdh::diffie_hellman(
            self.inner.as_nonzero_scalar(),
            other_pubkey.as_affine(),
        );

        let mut secret_bytes = [0u8; AccessKey::size()];
        let secret_expansion = shared_secret.extract::<sha2::Sha384>(None);
        if secret_expansion.expand(&[], &mut secret_bytes).is_err() {
            unreachable!("secret_bytes will always have the correct length");
        }

        AccessKey::from(secret_bytes)
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.verifying_key().fingerprint()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SigningKeyError> {
        let private_key =
            SecretKey::from_bytes(bytes.into()).map_err(|_| SigningKeyError::InvalidBytes)?;
        let inner = ecdsa::SigningKey::from(private_key);

        Ok(Self { inner })
    }

    #[cfg(feature = "pem")]
    pub fn from_pkcs8_pem(pem: &str) -> Result<Self, SigningKeyError> {
        use ecdsa::elliptic_curve::pkcs8::DecodePrivateKey;

        let p384_key =
            SecretKey::from_pkcs8_pem(pem).map_err(|_| SigningKeyError::PemDecodingFailed)?;
        let inner = ecdsa::SigningKey::from(p384_key);

        Ok(Self { inner })
    }

    #[cfg(feature = "pem")]
    pub fn from_sec1_pem(pem: &str) -> Result<Self, SigningKeyError> {
        let p384_key =
            SecretKey::from_sec1_pem(pem).map_err(|_| SigningKeyError::PemDecodingFailed)?;
        let inner = ecdsa::SigningKey::from(p384_key);

        Ok(Self { inner })
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        let inner = ecdsa::SigningKey::<NistP384>::random(rng);
        Self { inner }
    }

    pub fn key_id(&self) -> KeyId {
        self.verifying_key().key_id()
    }

    pub fn to_bytes(&self) -> [u8; KEY_SIZE] {
        let private_key: SecretKey = self.inner.clone().into();
        let private_key_bytes = private_key.to_bytes();

        debug_assert!(private_key_bytes.len() == KEY_SIZE);
        let mut private_key = [0u8; KEY_SIZE];
        private_key.copy_from_slice(&private_key_bytes);

        private_key
    }

    #[cfg(feature = "pem")]
    pub fn to_pkcs8_pem(&self) -> Result<Zeroizing<String>, SigningKeyError> {
        use ecdsa::elliptic_curve::pkcs8::EncodePrivateKey;

        let private_key: SecretKey = self.inner.clone().into();

        let pem = private_key
            .to_pkcs8_pem(elliptic_curve::pkcs8::LineEnding::LF)
            .map_err(|_| SigningKeyError::PemEncodingFailed)?;

        Ok(pem)
    }

    #[cfg(feature = "pem")]
    pub fn to_sec1_pem(&self) -> Result<Zeroizing<String>, SigningKeyError> {
        let private_key: SecretKey = self.inner.clone().into();

        let pem = private_key
            .to_sec1_pem(elliptic_curve::pkcs8::LineEnding::LF)
            .map_err(|_| SigningKeyError::PemEncodingFailed)?;

        Ok(pem)
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey::from(*self.inner.verifying_key())
    }
}

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{SigningKey({:?})}}", self.key_id())
    }
}

impl RandomizedDigestSigner<sha2::Sha384, Signature> for SigningKey {
    fn try_sign_digest_with_rng(
        &self,
        rng: &mut impl CryptoRngCore,
        digest: sha2::Sha384,
    ) -> Result<Signature, ecdsa::signature::Error> {
        let signature: ecdsa::Signature<NistP384> = self.inner.sign_digest_with_rng(rng, digest);
        Ok(Signature::from(signature))
    }
}

// note(sstelfox): Its not worth capturing the 'elliptic_curve::Error' type as the result is always
// just 'CryptoError'.
#[derive(Debug, thiserror::Error)]
pub enum SigningKeyError {
    #[error("failed to load encoded signing key")]
    InvalidBytes,

    #[error("failed to decode private key from SEC1 encoded PEM")]
    PemDecodingFailed,

    #[error("failed to encode private key as PEM")]
    PemEncodingFailed,
}
