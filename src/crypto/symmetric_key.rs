use std::ops::Deref;

use rand::Rng;

use crate::crypto::encrypted_payload::EncryptedPayload;

pub(crate) struct SymmetricKey([u8; 32]);

impl SymmetricKey {
    pub(crate) fn encrypt(
        &self,
        plain: &[u8],
        aad: &[u8],
    ) -> Result<EncryptedPayload, SymmetricKeyError> {
        unimplemented!()
    }

    pub(crate) fn from_bytes(key: [u8; 32]) -> Self {
        Self(key)
    }

    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self(rng.gen())
    }
}

impl Deref for SymmetricKey {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SymmetricKeyError {
    #[error("encrypting failed: {0}")]
    EncryptionFailed(String),
}
