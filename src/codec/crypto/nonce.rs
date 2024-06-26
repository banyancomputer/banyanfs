use std::ops::Deref;

use chacha20poly1305::XNonce as ChaChaNonce;
use futures::{AsyncWrite, AsyncWriteExt};
use rand::Rng;
use winnow::{token::take, Parser};

use crate::codec::{ParserResult, Stream};

const NONCE_LENGTH: usize = 24;

#[derive(Clone)]
pub struct Nonce([u8; NONCE_LENGTH]);

impl Nonce {
    pub fn as_bytes(&self) -> &[u8; NONCE_LENGTH] {
        &self.0
    }

    pub fn from_bytes(data: &[u8; NONCE_LENGTH]) -> Self {
        Self(*data)
    }

    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }

    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self(rng.gen())
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (remaining, slice) = take(NONCE_LENGTH).parse_peek(input)?;

        let mut bytes = [0u8; NONCE_LENGTH];
        bytes.copy_from_slice(slice);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        NONCE_LENGTH
    }
}

impl Deref for Nonce {
    type Target = ChaChaNonce;

    fn deref(&self) -> &Self::Target {
        ChaChaNonce::from_slice(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_nonce_parsing() {
        let mut rng = rand::thread_rng();
        let input: [u8; NONCE_LENGTH + 4] = rng.gen();
        let (remaining, nonce) = Nonce::parse(Stream::new(&input)).unwrap();

        assert_eq!(remaining.into_inner(), &input[NONCE_LENGTH..]);
        assert_eq!(nonce.as_bytes(), &input[..NONCE_LENGTH]);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_nonce_parsing_stream_too_short() {
        let input = [0u8; NONCE_LENGTH - 1];
        let result = Nonce::parse(Stream::new(&input));
        assert!(matches!(result, Err(winnow::error::ErrMode::Incomplete(_))));
    }
}
