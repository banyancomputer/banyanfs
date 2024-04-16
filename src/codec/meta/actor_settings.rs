use futures::io::{AsyncWrite, AsyncWriteExt};
use winnow::binary::le_u8;
use winnow::token::take;
use winnow::Parser;

use crate::codec::crypto::VerifyingKey;
use crate::codec::header::AccessMask;
use crate::codec::meta::VectorClock;
use crate::codec::{ParserResult, Stream};

const SOFTWARE_AGENT_BYTE_STR_SIZE: usize = 63;

#[derive(Clone, Debug)]
pub struct ActorSettings {
    verifying_key: VerifyingKey,
    vector_clock: VectorClock,
    access_mask: AccessMask,
    agent: Vec<u8>,
}

impl ActorSettings {
    pub fn access(&self) -> AccessMask {
        self.access_mask.clone()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        overwrite_version: bool,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += self.verifying_key.encode(writer).await?;
        written_bytes += self.vector_clock.encode(writer).await?;
        written_bytes += self.access_mask.encode(writer).await?;

        let (len, bytes) = if overwrite_version {
            current_version_byte_str()
        } else {
            let agent_len = self.agent.len();
            if agent_len > SOFTWARE_AGENT_BYTE_STR_SIZE {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "invalid agent byte string length",
                ));
            }

            let mut full_agent = [0; SOFTWARE_AGENT_BYTE_STR_SIZE];
            full_agent.copy_from_slice(&self.agent);

            (agent_len as u8, full_agent)
        };

        writer.write_all(&[len]).await?;
        writer.write_all(&bytes).await?;
        written_bytes += 1 + SOFTWARE_AGENT_BYTE_STR_SIZE;

        Ok(written_bytes)
    }

    pub fn new(verifying_key: VerifyingKey, access_mask: AccessMask) -> Self {
        let vector_clock = VectorClock::initialize();

        let (len, agent_fixed) = current_version_byte_str();
        let agent = agent_fixed[..len as usize].to_vec();

        Self {
            verifying_key,
            access_mask,
            vector_clock,
            agent,
        }
    }

    pub fn parse_private(input: Stream) -> ParserResult<Self> {
        let (input, verifying_key) = VerifyingKey::parse(input)?;
        let (input, vector_clock) = VectorClock::parse(input)?;
        let (input, access_mask) = AccessMask::parse(input)?;

        let (input, agent_len) = le_u8.parse_peek(input)?;
        let (input, agent_fixed) = take(SOFTWARE_AGENT_BYTE_STR_SIZE).parse_peek(input)?;
        let agent = agent_fixed[..agent_len as usize].to_vec();

        let actor_settings = Self {
            verifying_key,
            vector_clock,
            access_mask,
            agent,
        };

        Ok((input, actor_settings))
    }

    pub const fn size() -> usize {
        VerifyingKey::size()
            + VectorClock::size()
            + AccessMask::size()
            + 1
            + SOFTWARE_AGENT_BYTE_STR_SIZE
    }

    pub fn vector_clock(&self) -> VectorClock {
        self.vector_clock.clone()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key.clone()
    }
}

fn current_version_byte_str() -> (u8, [u8; SOFTWARE_AGENT_BYTE_STR_SIZE]) {
    let new_agent = crate::version::user_agent_byte_str();
    let new_agent_len = new_agent.len();

    let mut full_agent = [0; SOFTWARE_AGENT_BYTE_STR_SIZE];
    full_agent[..new_agent_len].copy_from_slice(&new_agent);

    (new_agent.len() as u8, full_agent)
}
