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
        Self::from(crate::version::minimal_version().as_str())
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
        full_agent[..agent_len].copy_from_slice(&self.0);

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

    pub fn to_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.0.clone())
    }
}

impl From<&str> for UserAgent {
    fn from(val: &str) -> Self {
        let val_trimmed_len = val.len().min(SOFTWARE_AGENT_BYTE_STR_SIZE);
        Self(val.as_bytes()[..val_trimmed_len].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use winnow::Partial;

    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_user_agent_roundtrip() {
        let sample_version_str = "test/v1.0.0-g283abc2".to_string();
        let user_agent = UserAgent::from(sample_version_str.as_str());

        let mut buffer = Vec::with_capacity(64);
        let length = user_agent.encode(&mut buffer).await.expect("encoding");
        assert_eq!(length, UserAgent::size());

        let partial = Partial::new(buffer.as_slice());
        let (remaining, parsed_ua) = UserAgent::parse(partial).expect("round trip");
        assert!(remaining.is_empty());

        let parsed_ua_str = parsed_ua.to_string().expect("valid chars");
        assert_eq!(parsed_ua_str, sample_version_str);
    }
}
