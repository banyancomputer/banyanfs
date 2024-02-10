use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;

use crate::codec::header::BANYAN_FS_MAGIC;
use crate::codec::AsyncEncodable;

#[derive(Debug, PartialEq)]
pub struct IdentityHeader;

impl IdentityHeader {
    pub fn parse_with_magic(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, _magic) = banyanfs_magic_tag(input)?;
        let (input, version) = version_field(input)?;

        // Only version one is valid
        if version != 0x01 {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Tag)));
        }

        Ok((input, Self))
    }
}

fn banyanfs_magic_tag(input: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    tag(BANYAN_FS_MAGIC)(input)
}

fn version_field(input: &[u8]) -> nom::IResult<&[u8], u8> {
    let (input, version_byte) = take(1u8)(input)?;
    let version_byte = version_byte[0];

    // The specification indicates decoders SHOULD ignore this bit. We allow the consumers of the
    // library to enable a stricter parsing mode.
    let reserved = (version_byte & 0x80) >> 7;
    if cfg!(feature = "strict") && reserved != 0 {
        return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)));
    }

    let version = version_byte & 0x7f;
    Ok((input, version))
}

#[async_trait]
impl AsyncEncodable for IdentityHeader {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        pos: usize,
    ) -> std::io::Result<usize> {
        writer.write_all(BANYAN_FS_MAGIC).await?;
        writer.write_all(&[0x01]).await?;

        Ok(pos + BANYAN_FS_MAGIC.len() + 1)
    }
}

#[cfg(tests)]
mod tests {
    use super::*;

    use rand::Rng;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip() {
        // Manually construct a correct header according to the RFC
        let mut source = BANYAN_FS_MAGIC.to_vec();
        source.extend(&[0x01]);

        let parsed = IdentityHeader::parse_with_magic(&source).unwrap();
        assert_eq!(parsed, IdentityHeader);

        let mut encoded = Vec::new();
        parsed.encode(&mut encoded, 0).await.unwrap();

        asssert_eq!(source, encoded);
    }
}
