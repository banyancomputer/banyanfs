use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u8;

use crate::codec::ParserResult;

const CONTENT_OPTIONS_RESERVED_MASK: u8 = 0b1111_1000;

const CONTENT_OPTIONS_FILESYSTEM_BIT: u8 = 0b0000_0100;

const CONTENT_OPTIONS_MAINTENANCE_BIT: u8 = 0b0000_0010;

const CONTENT_OPTIONS_DATA_BIT: u8 = 0b0000_0001;

#[derive(Debug)]
pub struct ContentOptions {
    filesystem: bool,
    maintenance: bool,
    data: bool,
}

impl ContentOptions {
    pub fn data_only() -> Self {
        Self {
            filesystem: false,
            maintenance: false,
            data: true,
        }
    }

    pub fn everything() -> Self {
        Self {
            filesystem: true,
            maintenance: true,
            data: true,
        }
    }

    pub fn metadata() -> Self {
        Self {
            filesystem: true,
            maintenance: true,
            data: true,
        }
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut options: u8 = 0x00;

        if self.filesystem {
            options |= CONTENT_OPTIONS_FILESYSTEM_BIT;
        }

        if self.maintenance {
            options |= CONTENT_OPTIONS_MAINTENANCE_BIT;
        }

        if self.data {
            options |= CONTENT_OPTIONS_DATA_BIT;
        }

        writer.write_all(&[options]).await?;

        Ok(1)
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & CONTENT_OPTIONS_RESERVED_MASK != 0 {
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        let filesystem = byte & CONTENT_OPTIONS_FILESYSTEM_BIT != 0;
        let maintenance = byte & CONTENT_OPTIONS_MAINTENANCE_BIT != 0;
        let data = byte & CONTENT_OPTIONS_DATA_BIT != 0;

        let content_options = ContentOptions {
            filesystem,
            maintenance,
            data,
        };

        Ok((input, content_options))
    }

    pub fn include_filesystem(&self) -> bool {
        self.filesystem
    }

    pub const fn size() -> usize {
        1
    }
}
