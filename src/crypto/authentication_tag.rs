use std::ops::Deref;

use chacha20poly1305::Tag as ChaChaTag;
use nom::bytes::streaming::take;
use nom::combinator::all_consuming;
use nom::IResult;

#[derive(Clone, Debug)]
pub(crate) struct AuthenticationTag([u8; TAG_LENGTH]);

impl AuthenticationTag {
    pub(crate) fn as_bytes(&self) -> &[u8; TAG_LENGTH] {
        &self.0
    }

    pub(crate) fn empty() -> Self {
        Self([0u8; TAG_LENGTH])
    }

    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (remaining, slice) = take(TAG_LENGTH)(input)?;

        let mut bytes = [0u8; TAG_LENGTH];
        bytes.copy_from_slice(slice);

        Ok((remaining, Self(bytes)))
    }

    pub(crate) fn parse_complete(input: &[u8]) -> Result<Self, nom::Err<nom::error::Error<&[u8]>>> {
        let (_, tag) = all_consuming(Self::parse)(input)?;
        Ok(tag)
    }
}

impl Deref for AuthenticationTag {
    type Target = ChaChaTag;

    fn deref(&self) -> &Self::Target {
        ChaChaTag::from_slice(&self.0)
    }
}

impl From<[u8; TAG_LENGTH]> for AuthenticationTag {
    fn from(bytes: [u8; TAG_LENGTH]) -> Self {
        Self(bytes)
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
