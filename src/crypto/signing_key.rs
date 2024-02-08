use ecdsa::signature::rand_core::CryptoRngCore;
use ecdsa::signature::RandomizedDigestSigner;
use p384::NistP384;
use rand::{CryptoRng, Rng};
use sha2::Digest;

use crate::parser::crypto::{KeyId, Signature, VerifyingKey};

const KEY_SIZE: usize = 48;

pub(crate) struct SigningKey {
    inner: ecdsa::SigningKey<NistP384>,
}

impl SigningKey {
    pub(crate) fn generate(rng: &mut impl CryptoRngCore) -> Self {
        let inner = ecdsa::SigningKey::<NistP384>::random(rng);
        Self { inner }
    }

    pub(crate) fn key_id(&self) -> KeyId {
        self.verifying_key().key_id()
    }

    pub(crate) fn to_private_bytes(&self) -> [u8; KEY_SIZE] {
        let private_key_bytes = self.inner.to_bytes();

        let mut private_key = [0u8; KEY_SIZE];
        private_key.copy_from_slice(&private_key_bytes);

        private_key
    }

    pub(crate) fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey::from(*self.inner.verifying_key())
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
