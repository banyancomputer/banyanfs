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

pub(crate) const ALL_SETTINGS_MASK: u8 = 0b1110_0111;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessMask(u8);

impl AccessMask {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(&[self.0]).await?;
        Ok(1)
    }

    pub fn has_data_key(&self) -> bool {
        self.0 & DATA_KEY_PRESENT_BIT != 0
    }

    pub fn has_filesystem_key(&self) -> bool {
        self.0 & FILESYSTEM_KEY_PRESENT_BIT != 0
    }

    pub fn has_maintenance_key(&self) -> bool {
        self.0 & MAINTENANCE_KEY_PRESENT_BIT != 0
    }

    pub fn is_historical(&self) -> bool {
        self.0 & HISTORICAL_BIT != 0
    }

    pub fn is_owner(&self) -> bool {
        self.0 & OWNER_BIT != 0
    }

    pub fn is_protected(&self) -> bool {
        self.0 & PROTECTED_BIT != 0
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, byte) = le_u8.parse_peek(input)?;
        Ok((input, Self::from(byte)))
    }

    pub(crate) fn set_historical(&mut self, historical: bool) {
        if historical {
            self.0 |= HISTORICAL_BIT;
        } else {
            self.0 &= !HISTORICAL_BIT;
        }
    }

    pub(crate) fn set_owner(&mut self, owner: bool) {
        if owner {
            self.0 |= OWNER_BIT;
        } else {
            self.0 &= !OWNER_BIT;
        }
    }

    pub(crate) fn set_protected(&mut self, protected: bool) {
        if protected {
            self.0 |= PROTECTED_BIT;
        } else {
            self.0 &= !PROTECTED_BIT;
        }
    }

    pub const fn size() -> usize {
        1
    }
}

impl From<u8> for AccessMask {
    fn from(value: u8) -> Self {
        Self(value & ALL_SETTINGS_MASK)
    }
}
