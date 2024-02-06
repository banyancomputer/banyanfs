use std::ops::Deref;

use chacha20poly1305::XNonce as ChaChaNonce;
use nom::bytes::streaming::take;
use nom::IResult;
use rand::Rng;

const NONCE_LENGTH: usize = 24;

pub(crate) struct Nonce([u8; NONCE_LENGTH]);

impl Nonce {
    pub(crate) fn as_bytes(&self) -> &[u8; NONCE_LENGTH] {
        &self.0
    }

    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self(rng.gen())
    }

    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (remaining, slice) = take(NONCE_LENGTH)(input)?;

        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        nonce_bytes.copy_from_slice(slice);

        Ok((remaining, Self(nonce_bytes)))
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
    }

    #[test]
    fn test_nonce_parsing_stream_too_short() {
        let input = [0u8; NONCE_LENGTH - 1];
        let result = Nonce::parse(&input);
        assert!(matches!(result, Err(nom::Err::Incomplete(_))));
    }
}
