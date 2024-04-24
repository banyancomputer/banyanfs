use std::ops::Deref;

use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{binary::le_u16, Parser};

use crate::codec::{ParserResult, Stream};

/// Key IDs are short identifiers that allow for quick filtering of a large number of potential
/// keys to a few highly probable ones that may match the intended key. You should not rely on
/// these values to be collision-free, checking all the full key [`Fingerprint`] instances that
/// match this KeyId.
///
/// These are used in the format as part of the key blinding access control mechanism. Public keys
/// that have access to a particular key will be able to find themselves quickly among all the
/// associated keys, but can be denied being present as the u16 space is highly likely to have
/// collisions matching keys other than your own.
///
/// For canonical identification of a particular public/private key pair a [`Fingerprint`] or
/// [`ActorId`] is more appropriate but should be restricted in their use to only private or
/// encrypted references when the Drive itself is encrypted.
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

    pub(crate) fn parse(input: Stream) -> ParserResult<Self> {
        let (input, key_id) = le_u16.parse_peek(input)?;
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
