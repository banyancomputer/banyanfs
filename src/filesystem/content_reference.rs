use futures::{AsyncWrite, AsyncWriteExt};
use nom::multi::count;
use nom::number::streaming::{le_u16, le_u64};

use crate::codec::{BlockSize, Cid, ParserResult};

#[derive(Clone, Debug)]
pub struct ContentReference {
    data_block_cid: Cid,
    block_size: BlockSize,
    chunks: Vec<ContentLocation>,
}

impl ContentReference {
    pub(crate) fn data_block_cid(&self) -> Cid {
        self.data_block_cid.clone()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = self.data_block_cid.encode(writer).await?;
        written_bytes += self.block_size.encode(writer).await?;

        let chunks_count = self.chunks.len();
        if chunks_count > u16::MAX as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "too many chunks in content reference",
            ));
        }

        let chunks_count = chunks_count as u16;
        let chunk_count_bytes = chunks_count.to_le_bytes();
        writer.write_all(&chunk_count_bytes).await?;
        written_bytes += chunk_count_bytes.len();

        for chunk in &self.chunks {
            written_bytes += chunk.encode(writer).await?;
        }

        Ok(written_bytes)
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (remaining, data_block_cid) = Cid::parse(input)?;
        let (remaining, block_size) = BlockSize::parse(input)?;

        let (remaining, chunk_count) = le_u16(remaining)?;
        let (remaining, chunks) = ContentLocation::parse_many(remaining, chunk_count)?;

        let content_ref = Self {
            data_block_cid,
            block_size,
            chunks,
        };

        Ok((remaining, content_ref))
    }

    pub fn parse_many(input: &[u8], ref_count: u8) -> ParserResult<Vec<Self>> {
        count(Self::parse, ref_count as usize)(input)
    }

    pub fn size(&self) -> usize {
        let base_size = Cid::size() + BlockSize::size() + 2;
        let chunk_size = self.chunks.len() * ContentLocation::size();
        base_size + chunk_size
    }
}

#[derive(Clone, Debug)]
pub struct ContentLocation {
    content_cid: Cid,
    block_index: u64,
}

impl ContentLocation {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = self.content_cid.encode(writer).await?;

        let block_index_bytes = self.block_index.to_le_bytes();
        writer.write_all(&block_index_bytes).await?;
        written_bytes += block_index_bytes.len();

        Ok(written_bytes)
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (remaining, content_cid) = Cid::parse(input)?;
        let (remaining, block_index) = le_u64(input)?;

        let location = Self {
            content_cid,
            block_index,
        };

        Ok((remaining, location))
    }

    pub fn parse_many(input: &[u8], ref_count: u16) -> ParserResult<Vec<Self>> {
        count(Self::parse, ref_count as usize)(input)
    }

    pub const fn size() -> usize {
        Cid::size() + 8
    }
}
