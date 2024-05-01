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

    fn modify_bit(&mut self, mask: u8, set: bool) {
        if set {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, byte) = le_u8.parse_peek(input)?;

        let access_mask = Self::try_from(byte).map_err(|_| {
            winnow::error::ErrMode::Cut(winnow::error::ParserError::from_error_kind(
                &input,
                winnow::error::ErrorKind::Tag,
            ))
        })?;

        Ok((input, access_mask))
    }

    pub(crate) fn set_data_key_present(&mut self, value: bool) {
        self.modify_bit(DATA_KEY_PRESENT_BIT, value)
    }

    pub(crate) fn set_filesystem_key_present(&mut self, value: bool) {
        self.modify_bit(FILESYSTEM_KEY_PRESENT_BIT, value)
    }

    pub(crate) fn set_maintenance_key_present(&mut self, value: bool) {
        self.modify_bit(MAINTENANCE_KEY_PRESENT_BIT, value)
    }

    pub(crate) fn set_historical(&mut self, value: bool) {
        self.modify_bit(HISTORICAL_BIT, value)
    }

    #[allow(dead_code)]
    pub(crate) fn set_owner(&mut self, value: bool) {
        self.modify_bit(OWNER_BIT, value)
    }

    #[allow(dead_code)]
    pub(crate) fn set_protected(&mut self, value: bool) {
        self.modify_bit(PROTECTED_BIT, value)
    }

    pub const fn size() -> usize {
        1
    }
}

impl TryFrom<u8> for AccessMask {
    type Error = AccessMaskError;

    fn try_from(value: u8) -> Result<Self, AccessMaskError> {
        if (value & !ALL_SETTINGS_MASK) != 0 {
            return Err(AccessMaskError::InvalidValue(value));
        }

        Ok(Self(value & ALL_SETTINGS_MASK))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AccessMaskError {
    #[error("invalid access mask value: {0}")]
    InvalidValue(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setting_toggles() {
        let mut access = AccessMask::try_from(0).unwrap();

        assert!(!access.has_data_key());
        access.set_data_key_present(true);
        assert!(access.has_data_key());
        access.set_data_key_present(false);
        assert!(!access.has_data_key());

        assert!(!access.has_filesystem_key());
        access.set_filesystem_key_present(true);
        assert!(access.has_filesystem_key());
        access.set_filesystem_key_present(false);
        assert!(!access.has_filesystem_key());

        assert!(!access.has_maintenance_key());
        access.set_maintenance_key_present(true);
        assert!(access.has_maintenance_key());
        access.set_maintenance_key_present(false);
        assert!(!access.has_maintenance_key());

        assert!(!access.is_historical());
        access.set_historical(true);
        assert!(access.is_historical());
        access.set_historical(false);
        assert!(!access.is_historical());

        assert!(!access.is_owner());
        access.set_owner(true);
        assert!(access.is_owner());
        access.set_owner(false);
        assert!(!access.is_owner());

        assert!(!access.is_protected());
        access.set_protected(true);
        assert!(access.is_protected());
        access.set_protected(false);
        assert!(!access.is_protected());
    }
}
