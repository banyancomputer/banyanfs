mod builder;

pub use builder::AccessMaskBuilder;

use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{binary::le_u8, Parser};

use crate::codec::{ParserResult, Stream};

pub(crate) const PROTECTED_BIT: u8 = 0b1000_0000;

pub(crate) const OWNER_BIT: u8 = 0b0100_0000;

pub(crate) const HISTORICAL_BIT: u8 = 0b0010_0000;

pub(crate) const FILESYSTEM_KEY_PRESENT_BIT: u8 = 0b0000_0100;

pub(crate) const DATA_KEY_PRESENT_BIT: u8 = 0b0000_0010;

pub(crate) const MAINTENANCE_KEY_PRESENT_BIT: u8 = 0b0000_0001;

#[derive(Clone, Debug)]
pub struct AccessMask {
    protected: bool,
    owner: bool,
    historical: bool,

    filesystem_key_present: bool,
    data_key_present: bool,
    maintenance_key_present: bool,
}

impl AccessMask {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut settings: u8 = 0x00;

        if self.protected {
            settings |= PROTECTED_BIT;
        }

        if self.owner {
            settings |= OWNER_BIT;
        }

        if self.historical {
            settings |= HISTORICAL_BIT;
        }

        if self.filesystem_key_present {
            settings |= FILESYSTEM_KEY_PRESENT_BIT;
        }

        if self.data_key_present {
            settings |= DATA_KEY_PRESENT_BIT;
        }

        if self.maintenance_key_present {
            settings |= MAINTENANCE_KEY_PRESENT_BIT;
        }

        writer.write_all(&[settings]).await?;

        Ok(1)
    }

    pub fn has_data_key(&self) -> bool {
        self.data_key_present
    }

    pub fn has_filesystem_key(&self) -> bool {
        self.filesystem_key_present
    }

    pub fn has_maintenance_key(&self) -> bool {
        self.maintenance_key_present
    }

    pub fn is_historical(&self) -> bool {
        self.historical
    }

    pub fn is_owner(&self) -> bool {
        self.owner
    }

    pub fn is_protected(&self) -> bool {
        self.protected
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, byte) = le_u8.parse_peek(input)?;
        Ok((input, Self::from(byte)))
    }

    pub const fn size() -> usize {
        1
    }
}

impl From<u8> for AccessMask {
    fn from(value: u8) -> Self {
        Self {
            protected: value & PROTECTED_BIT != 0,
            owner: value & OWNER_BIT != 0,
            historical: value & HISTORICAL_BIT != 0,

            filesystem_key_present: value & FILESYSTEM_KEY_PRESENT_BIT != 0,
            data_key_present: value & DATA_KEY_PRESENT_BIT != 0,
            maintenance_key_present: value & MAINTENANCE_KEY_PRESENT_BIT != 0,
        }
    }
}
