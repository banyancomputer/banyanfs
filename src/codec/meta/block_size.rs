use nom::number::streaming::le_u8;

use crate::codec::ParserResult;
use futures::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone, Copy)]
pub struct BlockSize {
    /// The power of two exponent representing the total size of the block including metadata,
    /// format overhead, and error blocks.
    total_space: u8,

    /// The power of two exponent representing the encrypted chunk size within the block. Must be
    /// the same or smaller than the total space.
    chunk_size: u8,
}

impl BlockSize {
    pub fn chunk_size(&self) -> u64 {
        2u64.pow(self.chunk_size as u32)
    }

    pub fn chunk_count(&self) -> u64 {
        2u64.pow((self.total_space - self.chunk_size) as u32)
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer
            .write_all(&[self.total_space, self.chunk_size])
            .await?;
        Ok(2)
    }

    /// Create a new instance of a BlockSize. Not exposed intentionally to limit the block sizes in
    /// use in the wild at this point in time.
    fn new(total_space: u8, chunk_size: u8) -> Result<Self, BlockSizeError> {
        if chunk_size > total_space {
            return Err(BlockSizeError::ChunkSizeTooLarge(chunk_size, total_space));
        }

        Ok(Self {
            total_space,
            chunk_size,
        })
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, total_space) = le_u8(input)?;
        let (input, chunk_size) = le_u8(input)?;

        let block_size = Self {
            total_space,
            chunk_size,
        };

        Ok((input, block_size))
    }

    pub const fn size() -> usize {
        2
    }

    pub fn small() -> Result<Self, BlockSizeError> {
        Self::new(18, 18)
    }

    pub fn standard() -> Result<Self, BlockSizeError> {
        Self::new(26, 20)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlockSizeError {
    #[error("chunk size {0} is larger than total space {1}")]
    ChunkSizeTooLarge(u8, u8),

    #[error("attempted to add a chunk of size {0} to a block with max size of {1}")]
    ChunkTooLarge(usize, usize),
}
