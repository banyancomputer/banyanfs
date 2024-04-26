use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{
    binary::{le_u64, le_u8},
    Parser,
};

use crate::codec::{ParserResult, Stream};

/// Data blocks come in two varieties, [`BlockKind::IndirectReference`] should only be used when the
/// data attempting to be stored exceeds the maximum capacity of a single FileContent struct to
/// reference directly as ContentReference (255 children). The full capacity varies based on the
/// configured BlockSize in use. At maximum BlockSize capacity, each ContentReference can store
/// $2^255 * 255$ bytes of data (less some overhead).
///
/// Blocks at the absolute maximum limit are unwieldly and recommended against. The standard block
/// sizes defined by this library allows for $2^26 * 255$ which allows individual files up to about
/// 16GiB, anything below this should generally use a [`BlockKind::Data`]. For larger single file
/// entries, you can instead use an [`BlockKind::IndirectReference`] block which instead of
/// containing data, allows increasing the limit of ContentLocation instances from 255 to the
/// number that can fit inside the configured BlockSize.
///
/// As of right now, a [`crate::codec::meta::BlockSize::small()`] can store 256KiB (ignoring some
/// overhead), and ContentLocation Data block references take up 41 bytes allowing 6,393 entries
/// allowing up to close to 400TiB in a single file entry. More can be used by using additional
/// indirect blocks or a large block size for the indirect reference block.
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

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, kind) = le_u8.parse_peek(input)?;

        match kind {
            0x00 => Ok((input, Self::Data)),
            0x01 => {
                let (input, total_size) = le_u64.parse_peek(input)?;
                Ok((input, Self::IndirectReference { total_size }))
            }
            _ => {
                let err = winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Verify,
                );
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
