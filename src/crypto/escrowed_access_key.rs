use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::XChaCha20Poly1305;
use nom::number::complete::be_u32;

use crate::crypto::{AccessKey, Nonce};

pub(crate) struct EscrowedAccessKey {
    nonce: Nonce,
    cipher_text: [u8; 36],
    tag: [u8; 16],
}

impl EscrowedAccessKey {
    pub(crate) fn assemble(nonce: Nonce, cipher_text: [u8; 36], tag: [u8; 16]) -> Self {
        Self {
            nonce,
            cipher_text,
            tag,
        }
    }

    pub(crate) fn decrypt(
        &self,
        key: &AccessKey,
        aad: &[u8],
    ) -> Result<AccessKey, EncryptedPayloadError> {
        let payload = Payload {
            msg: &self.cipher_text,
            aad,
        };

        let result = XChaCha20Poly1305::new(key)
            .decrypt(&self.nonce, payload)
            .map_err(|_| EncryptedPayloadError::CryptoFailure)?;

        // This is initialized to all 0xff so we can't mistake bad data for a correct validation
        // signal
        let mut result_bytes: [u8; 36] = [255; 36];
        result_bytes.copy_from_slice(&result[..]);

        let validation_pattern = match be_u32::<&[u8], ()>(&result_bytes[32..]) {
            Ok((_, vp)) => vp,
            _ => return Err(EncryptedPayloadError::BadValidationData),
        };

        if validation_pattern != 0 {
            return Err(EncryptedPayloadError::IncorrectKey);
        }

        let mut key: [u8; 32] = [0; 32];
        key.copy_from_slice(&result_bytes[0..32]);

        Ok(AccessKey::from_bytes(key))
    }

    pub(crate) fn to_bytes(&self) -> [u8; 148] {
        let mut bytes = [0u8; 148];

        bytes[0..96].copy_from_slice(self.nonce.as_bytes());
        bytes[96..132].copy_from_slice(&self.cipher_text);
        bytes[132..148].copy_from_slice(&self.tag);

        bytes
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EncryptedPayloadError {
    #[error("failed to read validation pattern")]
    BadValidationData,

    /// I would love this to be more descriptive, but the underlying library deliberately opaques
    /// the failure reason to avoid potential side-channel leakage.
    #[error("failed to decrypt data")]
    CryptoFailure,

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,
}
