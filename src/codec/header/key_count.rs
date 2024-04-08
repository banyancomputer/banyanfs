use std::ops::Deref;

use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{bytes::take, Parser};

use crate::codec::{ParserResult, Stream};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyCount(u8);

impl KeyCount {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(&[self.0]).await?;
        Ok(1)
    }
    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, count) = take(1u8).parse_next(input)?;
        Ok((input, Self(count[0])))
    }

    pub const fn size() -> usize {
        1
    }
}

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

impl TryFrom<usize> for KeyCount {
    type Error = std::io::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value > u8::MAX as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "invalid number of keys",
            ));
        }

        Ok(Self(value as u8))
    }
}
