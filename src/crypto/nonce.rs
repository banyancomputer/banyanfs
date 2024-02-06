use std::ops::Deref;

use chacha20poly1305::XNonce as ChaChaNonce;
use nom::bytes::streaming::take;
use nom::combinator::{map_res, verify};
use nom::error::{Error, ErrorKind, ParseError};
use nom::IResult;
use rand::Rng;

pub(crate) struct Nonce([u8; 24]);

impl Nonce {
    pub(crate) fn as_bytes(&self) -> &[u8; 24] {
        &self.0
    }

    pub(crate) fn from_slice(input: &[u8]) -> Result<Self, NonceError<&[u8]>> {
        if input.len() < 24 {
            return Err(NonceError::InvalidLength);
        }

        let mut nonce = [0; 24];
        nonce.copy_from_slice(input);

        Ok(Self(nonce))
    }

    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self(rng.gen())
    }

    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map_res(take(24u8), Self::from_slice)(input)
    }
}

impl Deref for Nonce {
    type Target = ChaChaNonce;

    fn deref(&self) -> &Self::Target {
        ChaChaNonce::from_slice(&self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum NonceError<I> {
    #[error("invalid nonce length")]
    InvalidLength,

    #[error("nom parsing error: {0}")]
    NomError(#[from] Error<I>),
}

impl<I> ParseError<I> for NonceError<I> {
    fn append(_: I, _: ErrorKind, other: Self) -> Self {
        other
    }

    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        Self::NomError(Error::new(input, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_from_slice() {
        let mut rng = rand::thread_rng();
        let input: [u8; 24] = rng.gen();
        let nonce = Nonce::from_slice(&input).unwrap();
        assert_eq!(nonce.as_bytes(), &input);
    }

    #[test]
    fn test_nonce_from_slice_invalid_length() {
        let input = [0u8; 23];
        let nonce = Nonce::from_slice(&input);
        assert!(matches!(nonce, Err(NonceError::InvalidLength)));
    }

    #[test]
    fn test_nonce_parsing() {
        let mut rng = rand::thread_rng();
        let input: [u8; 24] = rng.gen();
        let (remaining, nonce) = Nonce::parse(&input).unwrap();
        assert_eq!(remaining, &[]);
        assert_eq!(nonce.as_bytes(), &input);
    }
}
