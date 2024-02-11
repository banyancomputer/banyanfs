use crate::codec::crypto::{AccessKey, AuthenticationTag, Nonce};
use chacha20poly1305::{AeadInPlace, KeyInit, XChaCha20Poly1305};

pub struct SymLockedAccessKey {
    pub(crate) nonce: Nonce,
    pub(crate) cipher_text: [u8; AccessKey::size()],
    pub(crate) tag: AuthenticationTag,
}

impl SymLockedAccessKey {
    pub fn unlock(
        &self,
        decryption_key: &AccessKey,
    ) -> Result<AccessKey, SymLockedAccessKeyError<&[u8]>> {
        let mut key_payload = self.cipher_text;

        let cipher = XChaCha20Poly1305::new(decryption_key.chacha_key());
        cipher.decrypt_in_place_detached(&self.nonce, &[], &mut key_payload, &self.tag)?;

        Ok(AccessKey::from(key_payload))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SymLockedAccessKeyError<I> {
    #[error("crypto error: {0}")]
    CryptoFailure(String),

    #[error("decoding data failed: {0}")]
    FormatFailure(#[from] nom::Err<nom::error::Error<I>>),

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,
}

impl<I> From<chacha20poly1305::Error> for SymLockedAccessKeyError<I> {
    fn from(err: chacha20poly1305::Error) -> Self {
        SymLockedAccessKeyError::CryptoFailure(err.to_string())
    }
}
