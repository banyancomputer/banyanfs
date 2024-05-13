use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{token::take, Parser};

use super::{ECC_PRESENT_BIT, ENCRYPTED_BIT};
use crate::codec::meta::BlockSize;
use crate::codec::{ParserResult, Stream};

#[derive(Clone, Copy, Debug)]
pub struct DataOptions {
    pub(super) ecc_present: bool,
    pub(super) encrypted: bool,
    pub(super) block_size: BlockSize,
}

impl DataOptions {
    pub fn block_size(&self) -> &BlockSize {
        &self.block_size
    }

    pub fn ecc_present(&self) -> bool {
        self.ecc_present
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut option_byte = 0u8;

        if self.ecc_present {
            option_byte |= ECC_PRESENT_BIT;
        }

        if self.encrypted {
            option_byte |= ENCRYPTED_BIT;
        }

        writer.write_all(&[option_byte]).await?;
        let block_size = self.block_size.encode(writer).await?;

        Ok(1 + block_size)
    }

    pub fn encrypted(&self) -> bool {
        self.encrypted
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, version_byte) = take(1u8).parse_peek(input)?;
        let option_byte = version_byte[0];

        let ecc_present = (option_byte & ECC_PRESENT_BIT) == ECC_PRESENT_BIT;
        let encrypted = (option_byte & ENCRYPTED_BIT) == ENCRYPTED_BIT;
        let (input, block_size) = BlockSize::parse(input)?;

        let data_options = DataOptions {
            ecc_present,
            encrypted,
            block_size,
        };

        Ok((input, data_options))
    }

    pub const fn size() -> usize {
        2
    }
}
