use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::codec::header::BANYAN_FS_MAGIC;
use crate::codec::AsyncEncodable;

pub struct IdentityHeader;

impl IdentityHeader {
    pub(crate) fn parse_with_magic(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, _magic) = banyan_fs_magic_tag(input)?;
        Ok((input, Self))
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

#[async_trait::async_trait]
impl AsyncEncodable for IdentityHeader {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> tokio::io::Result<usize> {
        writer.write_all(BANYAN_FS_MAGIC).await?;
        writer.write_u8(0x01).await?;

        Ok(start_pos + BANYAN_FS_MAGIC.len() + 1)
    }
}
