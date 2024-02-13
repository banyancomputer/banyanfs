use std::ops::Deref;

use nom::bytes::streaming::take;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyCount(u8);

impl Deref for KeyCount {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u8> for KeyCount {
    fn from(count: u8) -> Self {
        Self(count)
    }
}

impl KeyCount {
    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, count) = take(1u8)(input)?;
        Ok((input, Self(count[0])))
    }
}
