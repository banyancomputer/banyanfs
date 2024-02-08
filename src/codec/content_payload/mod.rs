use nom::number::streaming::le_u8;
use nom::IResult;

use crate::codec::crypto::AccessKey;
use crate::crypto::SigningKey;

pub(crate) enum ContentPayload {
    Private,
    Public,
}

impl ContentPayload {
    pub(crate) fn parse_private<'a>(input: &'a [u8], key: &SigningKey) -> IResult<&'a [u8], Self> {
        let _key_id = key.key_id();
        let (input, key_count) = le_u8(input)?;
        let (input, _escrowed_keys) = AccessKey::parse_many(input, key_count)?;
        Ok((input, ContentPayload::Private))
    }

    pub(crate) fn parse_public(input: &[u8]) -> IResult<&[u8], Self> {
        Ok((input, ContentPayload::Public))
    }
}
