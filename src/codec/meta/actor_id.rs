use futures::AsyncWrite;

use crate::codec::crypto::{Fingerprint, KeyId};
use crate::codec::ParserResult;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct ActorId(Fingerprint);

impl ActorId {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        self.0.encode(writer).await
    }

    pub fn key_id(&self) -> KeyId {
        self.0.key_id()
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (remaining, fingerprint) = Fingerprint::parse(input)?;
        Ok((remaining, ActorId(fingerprint)))
    }

    pub const fn size() -> usize {
        Fingerprint::size()
    }
}

impl From<Fingerprint> for ActorId {
    fn from(fingerprint: Fingerprint) -> Self {
        Self(fingerprint)
    }
}
