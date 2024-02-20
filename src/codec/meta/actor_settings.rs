use futures::io::AsyncWrite;

use crate::codec::crypto::VerifyingKey;
use crate::codec::header::KeyAccessSettings;
use crate::codec::meta::VectorClock;
use crate::codec::{AsyncEncodable, ParserResult};

#[derive(Debug)]
pub struct ActorSettings {
    verifying_key: VerifyingKey,
    vector_clock: VectorClock,
    access_settings: KeyAccessSettings,
}

impl ActorSettings {
    pub fn actor_settings(&self) -> KeyAccessSettings {
        self.access_settings.clone()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += self.verifying_key.encode(writer).await?;
        written_bytes += self.vector_clock.encode(writer).await?;
        written_bytes += self.access_settings.encode(writer).await?;

        Ok(written_bytes)
    }

    pub fn new(verifying_key: VerifyingKey, access_settings: KeyAccessSettings) -> Self {
        let vector_clock = VectorClock::init();

        Self {
            verifying_key,
            access_settings,
            vector_clock,
        }
    }

    pub fn parse_private(input: &[u8]) -> ParserResult<Self> {
        let (input, verifying_key) = VerifyingKey::parse(input)?;
        let (input, vector_clock) = VectorClock::parse(input)?;
        let (input, access_settings) = KeyAccessSettings::parse_private(input)?;

        let actor_settings = Self {
            verifying_key,
            vector_clock,
            access_settings,
        };

        Ok((input, actor_settings))
    }

    pub const fn size() -> usize {
        VerifyingKey::size() + VectorClock::size() + KeyAccessSettings::size()
    }

    pub fn vector_clock(&self) -> VectorClock {
        self.vector_clock.clone()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key.clone()
    }
}
