use std::ops::{Deref, DerefMut};

use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use std::io::{Error as StdError, ErrorKind as StdErrorKind};
use winnow::token::take;
use winnow::Parser;

use crate::codec::crypto::{AccessKey, AuthenticationTag, Nonce};
use crate::codec::{ParserResult, Stream};

#[derive(Default)]
pub(crate) struct EncryptedBuffer {
    inner: Vec<u8>,
}

impl EncryptedBuffer {
    pub fn parse_and_decrypt<'a>(
        input: Stream<'a>,
        payload_size: usize,
        authenticated_data: &[u8],
        access_key: &AccessKey,
    ) -> ParserResult<'a, Vec<u8>> {
        let (input, nonce) = Nonce::parse(input)?;
        let (input, encrypted_slice) = take(payload_size).parse_peek(input)?;
        let (input, tag) = AuthenticationTag::parse(input)?;

        let mut buffer = encrypted_slice.to_vec();

        if let Err(err) = access_key.decrypt_buffer(nonce, authenticated_data, &mut buffer, tag) {
            tracing::error!("failed to decrypt permission buffer: {err}");
            let err = winnow::error::ParserError::from_error_kind(
                &input,
                winnow::error::ErrorKind::Verify,
            );
            return Err(winnow::error::ErrMode::Cut(err));
        }

        Ok((input, buffer))
    }

    pub(crate) async fn encrypt_and_encode<'a, W: 'a + AsyncWrite + Unpin + Send>(
        mut self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        authenticated_data: &[u8],
        access_key: &AccessKey,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let (nonce, tag) = access_key
            .encrypt_buffer(rng, authenticated_data, &mut self.inner)
            .map_err(|_| StdError::new(StdErrorKind::Other, "unable to encrypt filesystem"))?;

        written_bytes += nonce.encode(writer).await?;

        writer.write_all(&self.inner).await?;
        written_bytes += self.inner.len();

        written_bytes += tag.encode(writer).await?;

        Ok(written_bytes)
    }

    pub(crate) fn encrypted_len(&self) -> usize {
        Nonce::size() + self.inner.len() + AuthenticationTag::size()
    }
}

impl Deref for EncryptedBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for EncryptedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<Vec<u8>> for EncryptedBuffer {
    fn from(buffer: Vec<u8>) -> Self {
        Self { inner: buffer }
    }
}
