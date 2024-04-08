use futures::{AsyncWrite, AsyncWriteExt};
use winnow::binary::{le_u16, le_u64};
use winnow::combinator::repeat;
use winnow::Parser;

use crate::codec::filesystem::BlockKind;
use crate::codec::{BlockSize, Cid, ParserResult, Stream};

#[derive(Clone, Debug)]
pub struct ContentReference {
    data_block_cid: Cid,
    block_size: BlockSize,
    chunks: Vec<ContentLocation>,
}

impl ContentReference {
    pub(crate) fn new(
        data_block_cid: Cid,
        block_size: BlockSize,
        chunks: Vec<ContentLocation>,
    ) -> Self {
        Self {
            data_block_cid,
            block_size,
            chunks,
        }
    }

    pub(crate) fn chunks(&self) -> &[ContentLocation] {
        &self.chunks
    }

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

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, data_block_cid) = Cid::parse(input)?;
        let (input, block_size) = BlockSize::parse(input)?;

        let (input, chunk_count) = le_u16(input)?;
        let (input, chunks) = ContentLocation::parse_many(input, chunk_count)?;

        let content_ref = Self {
            data_block_cid,
            block_size,
            chunks,
        };

        Ok((input, content_ref))
    }

    pub fn parse_many(input: Stream, ref_count: u8) -> ParserResult<Vec<Self>> {
        repeat(ref_count as usize, Self::parse).parse_next(input)
    }

    pub fn size(&self) -> usize {
        let base_size = Cid::size() + BlockSize::size() + 2;
        let chunk_size = self.chunks.iter().map(ContentLocation::size).sum::<usize>();
        base_size + chunk_size
    }
}

#[derive(Clone, Debug)]
pub struct ContentLocation {
    block_kind: BlockKind,
    content_cid: Cid,
    block_index: u64,
}

impl ContentLocation {
    pub fn block_index(&self) -> u64 {
        self.block_index
    }

    pub fn block_kind(&self) -> &BlockKind {
        &self.block_kind
    }

    pub fn data(cid: Cid, block_index: u64) -> Self {
        Self {
            block_kind: BlockKind::Data,
            content_cid: cid,
            block_index,
        }
    }

    #[allow(dead_code)]
    pub fn content_cid(&self) -> &Cid {
        &self.content_cid
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = self.block_kind.encode(writer).await?;
        written_bytes += self.content_cid.encode(writer).await?;

        let block_index_bytes = self.block_index.to_le_bytes();
        writer.write_all(&block_index_bytes).await?;
        written_bytes += block_index_bytes.len();

        Ok(written_bytes)
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, block_kind) = BlockKind::parse(input)?;
        let (input, content_cid) = Cid::parse(input)?;
        let (input, block_index) = le_u64(input)?;

        let location = Self {
            block_kind,
            content_cid,
            block_index,
        };

        Ok((input, location))
    }

    pub fn parse_many(input: Stream, ref_count: u16) -> ParserResult<Vec<Self>> {
        repeat(ref_count as usize, Self::parse).parse_next(input)
    }

    pub fn size(&self) -> usize {
        Cid::size() + 8 + self.block_kind.size()
    }
}
