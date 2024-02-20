use async_trait::async_trait;
use futures::AsyncWrite;

use crate::codec::crypto::{Fingerprint, KeyId};
use crate::codec::AsyncEncodable;

// todo(sstelfox) likely need a vector clock here...
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct ActorId(Fingerprint);

impl ActorId {
    pub fn key_id(&self) -> KeyId {
        self.0.key_id()
    }

    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, fingerprint) = Fingerprint::parse(input)?;
        Ok((remaining, ActorId(fingerprint)))
    }
}

impl From<Fingerprint> for ActorId {
    fn from(fingerprint: Fingerprint) -> Self {
        Self(fingerprint)
    }
}

#[async_trait]
impl AsyncEncodable for ActorId {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        self.0.encode(writer).await
    }
}
