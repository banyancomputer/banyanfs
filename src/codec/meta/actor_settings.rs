use futures::io::AsyncWrite;

use crate::codec::crypto::VerifyingKey;
use crate::codec::header::AccessMask;
use crate::codec::meta::{UserAgent, VectorClock};
use crate::codec::{ParserResult, Stream};

#[derive(Clone, Debug)]
pub struct ActorSettings {
    verifying_key: VerifyingKey,
    vector_clock: VectorClock,
    access_mask: AccessMask,
    user_agent: UserAgent,
}

impl ActorSettings {
    pub fn access(&self) -> AccessMask {
        self.access_mask.clone()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += self.verifying_key.encode(writer).await?;
        written_bytes += self.vector_clock.encode(writer).await?;
        written_bytes += self.access_mask.encode(writer).await?;
        written_bytes += self.user_agent().encode(writer).await?;

        Ok(written_bytes)
    }

    pub fn new(verifying_key: VerifyingKey, access_mask: AccessMask) -> Self {
        let vector_clock = VectorClock::initialize();
        let user_agent = UserAgent::current();

        Self {
            verifying_key,
            access_mask,
            vector_clock,
            user_agent,
        }
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, verifying_key) = VerifyingKey::parse(input)?;
        let (input, vector_clock) = VectorClock::parse(input)?;
        let (input, access_mask) = AccessMask::parse(input)?;
        let (input, user_agent) = UserAgent::parse(input)?;

        let actor_settings = Self {
            verifying_key,
            vector_clock,
            access_mask,
            user_agent,
        };

        Ok((input, actor_settings))
    }

    pub const fn size() -> usize {
        VerifyingKey::size() + VectorClock::size() + AccessMask::size() + UserAgent::size()
    }

    pub fn update_user_agent(&mut self) {
        self.user_agent = UserAgent::current();
    }

    pub fn user_agent(&self) -> UserAgent {
        self.user_agent.clone()
    }

    pub fn vector_clock(&self) -> VectorClock {
        self.vector_clock.clone()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key.clone()
    }
}
