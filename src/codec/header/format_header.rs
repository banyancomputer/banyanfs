use async_trait::async_trait;
use futures::AsyncWrite;
use nom::sequence::tuple;

use crate::codec::AsyncEncodable;

use crate::codec::header::{IdentityHeader, PublicSettings};
use crate::filesystem::FilesystemId;

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
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> std::io::Result<usize> {
        let start_pos = IdentityHeader::encode(&IdentityHeader, writer, start_pos).await?;
        let start_pos = self.filesystem_id.encode(writer, start_pos).await?;

        let settings = PublicSettings::new(self.ecc_present, self.private);
        let start_pos = settings.encode(writer, start_pos).await?;

        Ok(start_pos)
    }
}
