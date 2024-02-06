use nom::bytes::complete::take;
use nom::error::ErrorKind;
use nom::number::complete::le_u32;
use nom::sequence::tuple;

use crate::crypto::utils::symmetric_decrypt;
use crate::crypto::{AccessKey, AuthenticationTag, CryptoError, Nonce};

pub(crate) struct EscrowedAccessKey {
    nonce: Nonce,
    cipher_text: [u8; 36],
    tag: AuthenticationTag,
}

impl EscrowedAccessKey {
    pub(crate) fn assemble(nonce: Nonce, cipher_text: [u8; 36], tag: AuthenticationTag) -> Self {
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
    ) -> Result<AccessKey, EncryptedPayloadError<&[u8]>> {
        let result = symmetric_decrypt(key, &self.nonce, &self.cipher_text, &self.tag, aad)
            .map_err(EncryptedPayloadError::CryptoFailure)?;

        let key = tuple((take(32usize), le_u32))(result.as_slice())
            .and_then(|(_, (key, suffix))| {
                if suffix == 0 {
                    Ok(key)
                } else {
                    Err(nom::Err::Error((&result, ErrorKind::Tag)))
                }
            })
            .unwrap();

        let mut fixed_key: [u8; 32] = [0u8; 32];
        fixed_key.copy_from_slice(&key);

        Ok(AccessKey::from_bytes(fixed_key))
    }

    pub(crate) fn to_bytes(&self) -> [u8; 148] {
        let mut bytes = [0u8; 148];
        let mut current_idx = 0;

        let nonce_bytes = self.nonce.as_bytes();
        let nonce_len = nonce_bytes.len();
        bytes[current_idx..(current_idx + nonce_len)].copy_from_slice(nonce_bytes);
        current_idx += nonce_len;

        let cipher_len = self.cipher_text.len();
        bytes[current_idx..(current_idx + cipher_len)].copy_from_slice(&self.cipher_text);
        current_idx += cipher_len;

        let tag_bytes = self.tag.as_bytes();
        let tag_len = tag_bytes.len();
        bytes[current_idx..(current_idx + tag_len)].copy_from_slice(tag_bytes);

        bytes
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EncryptedPayloadError<I> {
    #[error("decoding data failed: {0}")]
    FormatFailure(#[from] nom::Err<nom::error::Error<I>>),

    #[error("crypto helper error: {0}")]
    CryptoFailure(#[from] CryptoError),

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,
}
