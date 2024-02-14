use ecdsa::signature::rand_core::CryptoRngCore;
use ecdsa::signature::RandomizedDigestSigner;
use p384::NistP384;

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

    pub fn from_slice(bytes: &[u8]) -> Result<Self, SigningKeyError> {
        let inner = ecdsa::SigningKey::from_slice(&bytes).map_err(SigningKeyError::LoadFailed)?;
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
        let private_key_bytes = self.inner.to_bytes();

        let mut private_key = [0u8; KEY_SIZE];
        private_key.copy_from_slice(&private_key_bytes);

        private_key
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

#[derive(Debug, thiserror::Error)]
pub enum SigningKeyError {
    #[error("failed to load signing key: {0}")]
    LoadFailed(ecdsa::signature::Error),
}
