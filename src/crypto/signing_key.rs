use ecdsa::signature::rand_core::CryptoRngCore;
use ecdsa::signature::RandomizedDigestSigner;
use p384::NistP384;

use crate::codec::crypto::{KeyId, Signature, VerifyingKey};

const KEY_SIZE: usize = 48;

pub(crate) struct SigningKey {
    inner: ecdsa::SigningKey<NistP384>,
}

impl SigningKey {
    pub(crate) fn dh_exchange(&self, other_pubkey: &VerifyingKey) -> [u8; 32] {
        let shared_secret = elliptic_curve::ecdh::diffie_hellman(
            self.inner.as_nonzero_scalar(),
            other_pubkey.as_affine(),
        );

        let secret_expansion = shared_secret.extract::<sha2::Sha384>(None);

        let mut secret_bytes = [0u8; 32];
        if secret_expansion.expand(&[], &mut secret_bytes).is_err() {
            unreachable!("secret_bytes will always have the correct length");
        }

        secret_bytes
    }

    pub(crate) fn generate(rng: &mut impl CryptoRngCore) -> Self {
        let inner = ecdsa::SigningKey::<NistP384>::random(rng);
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn key_id(&self) -> KeyId {
        self.verifying_key().key_id()
    }

    #[allow(dead_code)]
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
