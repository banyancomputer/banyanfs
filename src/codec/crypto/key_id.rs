use std::ops::Deref;

use nom::number::streaming::le_u16;
use nom::IResult;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct KeyId(u16);

impl KeyId {
    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, key_id) = le_u16(input)?;
        Ok((input, Self(key_id)))
    }

    pub(crate) const fn size() -> usize {
        2
    }
}

impl Deref for KeyId {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u16> for KeyId {
    fn from(value: u16) -> Self {
        Self(value)
    }
}
