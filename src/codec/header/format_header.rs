use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::codec::AsyncEncodable;

use crate::codec::header::{FilesystemId, IdentityHeader, PublicSettings};

pub struct FormatHeader {
    pub ecc_present: bool,
    pub private: bool,
    pub filesystem_id: FilesystemId,
}

impl FormatHeader {
    pub(crate) fn parse_with_magic(input: &[u8]) -> nom::IResult<&[u8], Self> {
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

#[async_trait::async_trait]
impl AsyncEncodable for FormatHeader {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> tokio::io::Result<usize> {
        let start_pos = IdentityHeader::encode(&IdentityHeader, writer, start_pos).await?;
        let start_pos = self.filesystem_id.encode(writer, start_pos).await?;

        let settings = PublicSettings::new(self.ecc_present, self.private);
        let start_pos = settings.encode(writer, start_pos).await?;

        Ok(start_pos)
    }
}
