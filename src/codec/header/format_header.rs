use async_trait::async_trait;
use futures::AsyncWrite;
use nom::sequence::tuple;

use crate::codec::AsyncEncodable;

use crate::codec::header::{IdentityHeader, PublicSettings};
use crate::codec::FilesystemId;

#[derive(Debug, PartialEq)]
pub struct FormatHeader {
    pub ecc_present: bool,
    pub private: bool,
    pub filesystem_id: FilesystemId,
}

impl FormatHeader {
    pub fn parse_with_magic(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let mut header_parser = tuple((
            IdentityHeader::parse_with_magic,
            FilesystemId::parse,
            PublicSettings::parse,
        ));

        let (input, (_, filesystem_id, settings)) = header_parser(input)?;

        let header = FormatHeader {
            ecc_present: settings.ecc_present(),
            private: settings.private(),
            filesystem_id,
        };

        Ok((input, header))
    }
}

#[async_trait]
impl AsyncEncodable for FormatHeader {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += IdentityHeader::encode(&IdentityHeader, writer).await?;
        written_bytes += self.filesystem_id.encode(writer).await?;

        let settings = PublicSettings::new(self.ecc_present, self.private);
        written_bytes += settings.encode(writer).await?;

        Ok(written_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::Rng;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_public_round_trip() {
        let mut rng = crate::utils::crypto_rng();

        // Manually construct an IdentityHeader...
        let mut source = crate::codec::header::BANYAN_FS_MAGIC.to_vec();
        source.extend(&[0x01]);

        // Followed by a filesystem ID
        let raw_id: [u8; 16] = rng.gen();
        source.extend(&raw_id);
        let filesystem_id = FilesystemId::from(raw_id);

        // A public non-ECC header
        source.extend(&[0x00]);

        let (remaining, parsed) = FormatHeader::parse_with_magic(&source).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(
            parsed,
            FormatHeader {
                ecc_present: false,
                private: false,
                filesystem_id
            }
        );

        let mut encoded = Vec::new();
        let size = parsed.encode(&mut encoded).await.unwrap();

        assert_eq!(source, encoded);
        assert_eq!(source.len(), size);
    }
}
