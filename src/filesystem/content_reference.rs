use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::multi::count;
use nom::number::streaming::le_u32;

use crate::codec::{AsyncEncodable, Cid};

pub struct ContentReference {
    data_block_cid: Cid,
    offset: u32,
    length: u32,
}

impl ContentReference {
    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, data_block_cid) = Cid::parse(input)?;

        let (remaining, offset) = le_u32(remaining)?;
        let (remaining, length) = le_u32(remaining)?;

        let content_reference = Self {
            data_block_cid,
            offset,
            length,
        };

        Ok((remaining, content_reference))
    }

    pub fn parse_many(input: &[u8], ref_count: u8) -> nom::IResult<&[u8], Vec<Self>> {
        count(Self::parse, ref_count as usize)(input)
    }

    pub fn size(&self) -> u64 {
        self.length as u64
    }
}

#[async_trait]
impl AsyncEncodable for ContentReference {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        pos: usize,
    ) -> std::io::Result<usize> {
        let pos = self.data_block_cid.encode(writer, pos).await?;

        writer.write_all(&self.offset.to_le_bytes()).await?;
        writer.write_all(&self.length.to_le_bytes()).await?;

        Ok(pos + 8)
    }
}
