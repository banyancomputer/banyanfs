use chacha20poly1305::{AeadInPlace, KeyInit, XChaCha20Poly1305};
use futures::{AsyncWrite, AsyncWriteExt};
use winnow::token::take;
use winnow::{unpeek, Parser};

use crate::codec::crypto::{AccessKey, AuthenticationTag, Nonce, SigningKey, VerifyingKey};
use crate::codec::{ParserResult, Stream};

#[derive(Clone)]
pub struct AsymLockedAccessKey {
    pub(crate) dh_exchange_key: VerifyingKey,
    pub(crate) nonce: Nonce,
    pub(crate) cipher_text: [u8; AccessKey::size()],
    pub(crate) tag: AuthenticationTag,
}

impl AsymLockedAccessKey {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += self.dh_exchange_key.encode(writer).await?;
        written_bytes += self.nonce.encode(writer).await?;

        writer.write_all(&self.cipher_text).await?;
        written_bytes += self.cipher_text.len();

        written_bytes += self.tag.encode(writer).await?;

        Ok(written_bytes)
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, (dh_exchange_key, nonce, raw_cipher_text, tag)) = (
            unpeek(VerifyingKey::parse),
            unpeek(Nonce::parse),
            // This is NOT being parsed into the target data type yet as its still encrypted. We'll
            // construct it when the contents are valid.
            take(AccessKey::size()),
            unpeek(AuthenticationTag::parse),
        )
            .parse_peek(input)?;

        let mut cipher_text = [0u8; AccessKey::size()];
        cipher_text.copy_from_slice(raw_cipher_text);

        let access_key = Self {
            dh_exchange_key,
            nonce,
            cipher_text,
            tag,
        };

        Ok((input, access_key))
    }

    pub const fn size() -> usize {
        VerifyingKey::size() + Nonce::size() + AccessKey::size() + AuthenticationTag::size()
    }

    pub fn unlock(&self, key: &SigningKey) -> Result<AccessKey, AsymLockedAccessKeyError> {
        let shared_secret = key.dh_exchange(&self.dh_exchange_key);
        let mut key_payload = self.cipher_text;

        XChaCha20Poly1305::new(shared_secret.chacha_key()).decrypt_in_place_detached(
            &self.nonce,
            &[],
            &mut key_payload,
            &self.tag,
        )?;

        Ok(AccessKey::from(key_payload))
    }
}

impl std::fmt::Debug for AsymLockedAccessKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AsymLockedAccessKey(*encrytped*)")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AsymLockedAccessKeyError {
    #[error("crypto error: {0}")]
    CryptoFailure(String),

    #[error("decoding data failed: {0}")]
    FormatFailure(String),

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,
}

impl From<chacha20poly1305::Error> for AsymLockedAccessKeyError {
    fn from(err: chacha20poly1305::Error) -> Self {
        AsymLockedAccessKeyError::CryptoFailure(err.to_string())
    }
}
