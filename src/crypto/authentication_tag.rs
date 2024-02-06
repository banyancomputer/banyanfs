use std::ops::Deref;

use chacha20poly1305::Tag as ChaChaTag;
use nom::bytes::streaming::take;
use nom::IResult;

const TAG_LENGTH: usize = 16;

#[derive(Debug)]
pub(crate) struct AuthenticationTag([u8; TAG_LENGTH]);

impl AuthenticationTag {
    pub(crate) fn as_bytes(&self) -> &[u8; TAG_LENGTH] {
        &self.0
    }

    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (remaining, slice) = take(TAG_LENGTH)(input)?;

        let mut nonce_bytes = [0u8; TAG_LENGTH];
        nonce_bytes.copy_from_slice(slice);

        Ok((remaining, Self(nonce_bytes)))
    }
}

impl Deref for AuthenticationTag {
    type Target = ChaChaTag;

    fn deref(&self) -> &Self::Target {
        ChaChaTag::from_slice(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    #[test]
    fn test_authentication_tag_parsing() {
        let mut rng = rand::thread_rng();
        let input: [u8; TAG_LENGTH + 4] = rng.gen();
        let (remaining, tag) = AuthenticationTag::parse(&input).unwrap();

        assert_eq!(remaining, &input[TAG_LENGTH..]);
        assert_eq!(tag.as_bytes(), &input[..TAG_LENGTH]);
    }

    #[test]
    fn test_authentication_tag_parsing_stream_too_short() {
        let input = [0u8; TAG_LENGTH - 1];
        let result = AuthenticationTag::parse(&input);
        assert!(matches!(result, Err(nom::Err::Incomplete(_))));
    }
}
