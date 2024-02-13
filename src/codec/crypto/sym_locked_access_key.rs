use async_trait::async_trait;
use chacha20poly1305::{AeadInPlace, KeyInit, XChaCha20Poly1305};
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;

use crate::codec::crypto::{AccessKey, AuthenticationTag, Nonce};
use crate::codec::AsyncEncodable;

#[derive(Clone)]
pub struct SymLockedAccessKey {
    pub(crate) nonce: Nonce,
    pub(crate) cipher_text: [u8; AccessKey::size()],
    pub(crate) tag: AuthenticationTag,
}

impl SymLockedAccessKey {
    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, nonce) = Nonce::parse(input)?;

        let (remaining, cipher_text) = take(AccessKey::size())(remaining)?;
        let mut fixed_cipher_text = [0u8; AccessKey::size()];
        fixed_cipher_text.copy_from_slice(cipher_text);

        let (remaining, tag) = AuthenticationTag::parse(remaining)?;

        let parsed = Self {
            nonce,
            cipher_text: fixed_cipher_text,
            tag,
        };

        Ok((remaining, parsed))
    }

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

#[async_trait]
impl AsyncEncodable for SymLockedAccessKey {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut written_bytes = self.nonce.encode(writer).await?;

        writer.write_all(&self.cipher_text).await?;
        written_bytes += self.cipher_text.len();

        written_bytes += self.tag.encode(writer).await?;

        Ok(written_bytes)
    }
}

impl std::fmt::Debug for SymLockedAccessKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SymLockedAccessKey(*redacted*)")
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
