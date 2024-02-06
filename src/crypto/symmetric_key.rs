use std::ops::Deref;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key as ChaChaKey, XChaCha20Poly1305};
use rand::Rng;

use crate::crypto::{EncryptedPayload, Nonce};

pub(crate) struct SymmetricKey([u8; 32]);

impl SymmetricKey {
    pub(crate) fn encrypt(
        &self,
        nonce: &Nonce,
        msg: &[u8],
        aad: &[u8],
    ) -> Result<EncryptedPayload, SymmetricKeyError> {
        XChaCha20Poly1305::new(self)
            .encrypt(nonce, Payload { msg, aad })
            .map_err(|_| SymmetricKeyError::CryptoFailure)
            .map(EncryptedPayload::from_cipher_blob)
    }

    pub(crate) fn from_bytes(key: [u8; 32]) -> Self {
        Self(key)
    }

    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self(rng.gen())
    }
}

impl Deref for SymmetricKey {
    type Target = ChaChaKey;

    fn deref(&self) -> &Self::Target {
        ChaChaKey::from_slice(&self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SymmetricKeyError {
    /// I would love this to be more descriptive, but the underlying library deliberately opaques
    /// the failure reason to avoid potential side-channel leakage.
    #[error("failed to encrypt data")]
    CryptoFailure,
}
