use std::ops::Deref;

use async_trait::async_trait;
use chacha20poly1305::XNonce as ChaChaNonce;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::combinator::all_consuming;
use nom::IResult;
use rand::Rng;

use crate::codec::AsyncEncodable;

const NONCE_LENGTH: usize = 24;

#[derive(Clone)]
pub struct Nonce([u8; NONCE_LENGTH]);

impl Nonce {
    pub fn as_bytes(&self) -> &[u8; NONCE_LENGTH] {
        &self.0
    }

    #[allow(dead_code)]
    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self(rng.gen())
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (remaining, slice) = take(NONCE_LENGTH)(input)?;

        let mut bytes = [0u8; NONCE_LENGTH];
        bytes.copy_from_slice(slice);

        Ok((remaining, Self(bytes)))
    }

    pub fn parse_complete(input: &[u8]) -> Result<Self, nom::Err<nom::error::Error<&[u8]>>> {
        let (_, tag) = all_consuming(Self::parse)(input)?;
        Ok(tag)
    }

    pub const fn size() -> usize {
        NONCE_LENGTH
    }
}

#[async_trait]
impl AsyncEncodable for Nonce {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(start_pos + self.0.len())
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

    #[test]
    fn test_nonce_parsing() {
        let mut rng = rand::thread_rng();
        let input: [u8; NONCE_LENGTH + 4] = rng.gen();
        let (remaining, nonce) = Nonce::parse(&input).unwrap();

        assert_eq!(remaining, &input[NONCE_LENGTH..]);
        assert_eq!(nonce.as_bytes(), &input[..NONCE_LENGTH]);

        assert!(Nonce::parse_complete(&input).is_err());
        assert!(Nonce::parse_complete(&input[..NONCE_LENGTH]).is_ok());
    }

    #[test]
    fn test_nonce_parsing_stream_too_short() {
        let input = [0u8; NONCE_LENGTH - 1];
        let result = Nonce::parse(&input);
        assert!(matches!(result, Err(nom::Err::Incomplete(_))));
    }
}
