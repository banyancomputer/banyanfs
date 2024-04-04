use futures::{AsyncWrite, AsyncWriteExt};
use winnow::number::streaming::{le_u64, le_u8};

use crate::codec::ParserResult;

#[derive(Clone, Debug)]
pub enum BlockKind {
    Data,
    IndirectReference { total_size: u64 },
}

impl BlockKind {
    pub fn data() -> Self {
        Self::Data
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        match self {
            Self::Data => {
                writer.write_all(&[0x00]).await?;
            }
            Self::IndirectReference { total_size } => {
                writer.write_all(&[0x01]).await?;

                let size_bytes = total_size.to_le_bytes();
                writer.write_all(&size_bytes).await?;
            }
        }

        Ok(self.size())
    }

    pub fn indirect_reference(total_size: u64) -> Self {
        Self::IndirectReference { total_size }
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, kind) = le_u8(input)?;

        match kind {
            0x00 => Ok((input, Self::Data)),
            0x01 => {
                let (input, total_size) = le_u64(input)?;
                Ok((input, Self::IndirectReference { total_size }))
            }
            _ => {
                let err = winnow::error::ParseError::from_error_kind(input, winnow::error::ErrorKind::Verify);
                Err(winnow::error::ErrMode::Cut(err))
            }
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Data => 1,
            Self::IndirectReference { .. } => 9,
        }
    }
}
