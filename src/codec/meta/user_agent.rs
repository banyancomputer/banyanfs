use futures::io::{AsyncWrite, AsyncWriteExt};
use winnow::binary::le_u8;
use winnow::token::take;
use winnow::Parser;

use crate::codec::{ParserResult, Stream};

const SOFTWARE_AGENT_BYTE_STR_SIZE: usize = 63;

#[derive(Clone, Debug)]
pub struct UserAgent(Vec<u8>);

impl UserAgent {
    pub fn current() -> Self {
        let agent = crate::version::user_agent_byte_str();
        Self(agent[..63].to_vec())
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let agent_len = self.0.len();
        if agent_len > SOFTWARE_AGENT_BYTE_STR_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid agent byte string length",
            ));
        }

        let mut full_agent = [0; SOFTWARE_AGENT_BYTE_STR_SIZE];
        full_agent.copy_from_slice(&self.0);

        writer.write_all(&[agent_len as u8]).await?;
        writer.write_all(full_agent.as_slice()).await?;

        Ok(Self::size())
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, agent_len) = le_u8.parse_peek(input)?;
        let (input, agent_fixed) = take(SOFTWARE_AGENT_BYTE_STR_SIZE).parse_peek(input)?;

        let user_agent = agent_fixed[..agent_len as usize].to_vec();

        Ok((input, Self(user_agent)))
    }

    pub const fn size() -> usize {
        1 + SOFTWARE_AGENT_BYTE_STR_SIZE
    }
}
