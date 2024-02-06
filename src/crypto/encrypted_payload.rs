use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::XChaCha20Poly1305;

use crate::crypto::{Nonce, SymmetricKey};

pub(crate) struct EncryptedPayload(Vec<u8>);

impl EncryptedPayload {
    pub(crate) fn decrypt(
        &self,
        key: &SymmetricKey,
        nonce: &Nonce,
        aad: &[u8],
    ) -> Result<Vec<u8>, EncryptedPayloadError> {
        XChaCha20Poly1305::new(&key)
            .decrypt(nonce, Payload { msg: &self.0, aad })
            .map_err(|_| EncryptedPayloadError::CryptoFailure)
    }

    // todo: might want access to split cipher_text from tag which is the last 16 bytes. There is a
    // dedicate type available for the tags that could be used.

    pub(crate) fn from_cipher_blob(cipher_blob: Vec<u8>) -> Self {
        Self(cipher_blob)
    }

    pub(crate) fn to_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EncryptedPayloadError {
    /// I would love this to be more descriptive, but the underlying library deliberately opaques
    /// the failure reason to avoid potential side-channel leakage.
    #[error("failed to decrypt data")]
    CryptoFailure,
}
