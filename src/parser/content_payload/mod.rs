use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;
use nom::IResult;

mod access_key;

pub(crate) use access_key::AccessKey;

use crate::crypto::SigningKey;

pub(crate) enum ContentPayload {
    Private,
    Public,
}

impl ContentPayload {
    pub(crate) fn parse_private<'a>(input: &'a [u8], _key: &SigningKey) -> IResult<&'a [u8], Self> {
        let (input, key_count) = le_u8(input)?;
        let (input, _escrowed_keys) = AccessKey::parse_many(input, key_count)?;
        Ok((input, ContentPayload::Private))
    }

    pub(crate) fn parse_public(input: &[u8]) -> IResult<&[u8], Self> {
        Ok((input, ContentPayload::Public))
    }
}
