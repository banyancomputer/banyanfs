use futures::{AsyncWrite, AsyncWriteExt};
use winnow::bytes::streaming::{tag, take};

use crate::codec::header::BANYAN_FS_MAGIC;
use crate::codec::ParserResult;

#[derive(Debug, PartialEq)]
pub struct IdentityHeader;

impl IdentityHeader {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(BANYAN_FS_MAGIC).await?;
        writer.write_all(&[0x01]).await?;

        Ok(BANYAN_FS_MAGIC.len() + 1)
    }

    pub fn parse_with_magic(input: &[u8]) -> ParserResult<Self> {
        let (input, _magic) = banyanfs_magic_tag(input)?;
        let (input, version) = version_field(input)?;

        // Only version one is valid
        if version != 0x01 {
            let err = winnow::error::ParseError::from_error_kind(input, winnow::error::ErrorKind::Verify);
            return Err(winnow::error::ErrMode::Cut(err));
        }

        Ok((input, Self))
    }
}

fn banyanfs_magic_tag(input: &[u8]) -> ParserResult<&[u8]> {
    tag(BANYAN_FS_MAGIC)(input)
}

fn version_field(input: &[u8]) -> ParserResult<u8> {
    let (input, version_byte) = take(1u8)(input)?;
    let version_byte = version_byte[0];

    // The specification indicates decoders SHOULD ignore this bit. We allow the consumers of the
    // library to enable a stricter parsing mode.
    let reserved = (version_byte & 0x80) >> 7;
    if cfg!(feature = "strict") && reserved != 0 {
        let err = winnow::error::ParseError::from_error_kind(input, winnow::error::ErrorKind::Verify);
        return Err(winnow::error::ErrMode::Cut(err));
    }

    let version = version_byte & 0x7f;
    Ok((input, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip() {
        // Manually construct a correct header according to the RFC
        let mut source = BANYAN_FS_MAGIC.to_vec();
        source.extend(&[0x01]);

        let (remaining, parsed) = IdentityHeader::parse_with_magic(&source).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(parsed, IdentityHeader);

        let mut encoded = Vec::new();
        let size = parsed.encode(&mut encoded).await.unwrap();

        assert_eq!(source, encoded);
        assert_eq!(source.len(), size);
    }
}
