use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::le_u8;
use nom::IResult;

use crate::codec::AsyncEncodable;

const CONTENT_OPTIONS_RESERVED_MASK: u8 = 0b1110_0000;

const CONTENT_OPTIONS_REALIZED_VIEW_BIT: u8 = 0b0001_0000;

const CONTENT_OPTIONS_JOURNAL_BIT: u8 = 0b0000_1000;

const CONTENT_OPTIONS_JOURNAL_INDEX_BIT: u8 = 0b0000_0100;

const CONTENT_OPTIONS_MAINTENANCE_BIT: u8 = 0b0000_0010;

const CONTENT_OPTIONS_DATA_BIT: u8 = 0b0000_0001;

pub struct ContentOptions {
    realized_view: bool,
    journal: bool,
    journal_index: bool,
    maintenance: bool,
    data: bool,
}

impl ContentOptions {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & CONTENT_OPTIONS_RESERVED_MASK != 0 {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Tag)));
        }

        let realized_view = byte & CONTENT_OPTIONS_REALIZED_VIEW_BIT != 0;
        let journal = byte & CONTENT_OPTIONS_JOURNAL_BIT != 0;
        let journal_index = byte & CONTENT_OPTIONS_JOURNAL_INDEX_BIT != 0;
        let maintenance = byte & CONTENT_OPTIONS_MAINTENANCE_BIT != 0;
        let data = byte & CONTENT_OPTIONS_DATA_BIT != 0;

        let content_options = ContentOptions {
            realized_view,
            journal,
            journal_index,
            maintenance,
            data,
        };

        Ok((input, content_options))
    }
}

#[async_trait]
impl AsyncEncodable for ContentOptions {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        pos: usize,
    ) -> std::io::Result<usize> {
        let mut options: u8 = 0x00;

        if self.realized_view {
            options |= CONTENT_OPTIONS_REALIZED_VIEW_BIT;
        }

        if self.journal {
            options |= CONTENT_OPTIONS_JOURNAL_BIT;
        }

        if self.journal_index {
            options |= CONTENT_OPTIONS_JOURNAL_INDEX_BIT;
        }

        if self.maintenance {
            options |= CONTENT_OPTIONS_MAINTENANCE_BIT;
        }

        if self.data {
            options |= CONTENT_OPTIONS_DATA_BIT;
        }

        writer.write_all(&[options]).await?;

        Ok(pos + 1)
    }
}
