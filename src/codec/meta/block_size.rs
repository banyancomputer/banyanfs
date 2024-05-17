use winnow::{binary::le_u8, Parser};

use crate::codec::{ParserResult, Stream};
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
    pub const fn chunk_size(&self) -> u64 {
        2u64.pow(self.chunk_size as u32)
    }

    pub const fn chunk_count(&self) -> u64 {
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
    const fn new(total_space: u8, chunk_size: u8) -> Result<Self, BlockSizeError> {
        if chunk_size > total_space {
            return Err(BlockSizeError::ChunkSizeTooLarge(chunk_size, total_space));
        }

        Ok(Self {
            total_space,
            chunk_size,
        })
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, total_space) = le_u8.parse_peek(input)?;
        let (input, chunk_size) = le_u8.parse_peek(input)?;

        let block_size = Self {
            total_space,
            chunk_size,
        };

        Ok((input, block_size))
    }

    pub const fn size() -> usize {
        2
    }

    pub const fn small() -> Self {
        match Self::new(18, 18) {
            Err(_) => panic!("With known values this will never error"),
            Ok(res) => res,
        }
    }

    pub const fn standard() -> Self {
        match Self::new(26, 20) {
            Err(_) => panic!("With known values this will never error"),
            Ok(res) => res,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlockSizeError {
    #[error("chunk size {0} is larger than total space {1}")]
    ChunkSizeTooLarge(u8, u8),

    #[error("attempted to add a chunk of size {0} to a block with max size of {1}")]
    ChunkTooLarge(usize, usize),
}
