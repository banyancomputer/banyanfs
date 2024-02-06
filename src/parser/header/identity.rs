use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;

use crate::parser::header::BANYAN_FS_MAGIC;

pub(crate) struct IdentityHeader {
    header_length: u32,
}

impl IdentityHeader {
    pub(crate) fn parse_with_magic(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, (_, header_length)) = tuple((banyan_fs_magic_tag, le_u32))(input)?;
        let format_header = Self { header_length };
        Ok((input, format_header))
    }
}

fn banyan_fs_magic_tag(input: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    tag(BANYAN_FS_MAGIC)(input)
}

fn fs_version_one(input: &[u8]) -> nom::IResult<&[u8], ()> {
    let (input, version_byte) = take(1u8)(input)?;
    let version_byte = version_byte[0];

    // The specification indicates decoders SHOULD ignore this bit. We allow the consumers of the
    // library to enable a stricter parsing mode.
    let reserved = (version_byte & 0x80) >> 7;
    if cfg!(feature = "strict") && reserved != 0 {
        return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)));
    }

    let version = version_byte & 0x7f;
    if version == 0x01 {
        Ok((input, ()))
    } else {
        Err(nom::Err::Failure(NomError::new(input, ErrorKind::Tag)))
    }
}
