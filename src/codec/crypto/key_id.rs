use std::ops::Deref;

use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u16;

use crate::codec::ParserResult;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct KeyId(u16);

impl KeyId {
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let key_id_bytes = self.0.to_le_bytes();
        writer.write_all(&key_id_bytes).await?;
        Ok(key_id_bytes.len())
    }

    pub(crate) fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, key_id) = le_u16(input)?;
        Ok((input, Self(key_id)))
    }

    pub(crate) const fn size() -> usize {
        2
    }
}

impl std::fmt::Debug for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeyId(0x{:04x?})", self.0)
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
