use async_trait::async_trait;
use ecdsa::signature::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use rand::Rng;

use crate::codec::AsyncEncodable;

const PERMANENT_ID_SIZE: usize = 16;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PermanentId([u8; PERMANENT_ID_SIZE]);

impl PermanentId {
    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        Self(rng.gen())
    }

    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, id_bytes) = take(PERMANENT_ID_SIZE)(input)?;

        let mut bytes = [0u8; PERMANENT_ID_SIZE];
        bytes.copy_from_slice(id_bytes);

        Ok((remaining, Self(bytes)))
    }
}

#[async_trait]
impl AsyncEncodable for PermanentId {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }
}

impl std::fmt::Debug for PermanentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id_str: String = self
            .0
            .iter()
            .fold(String::new(), |acc, &b| format!("{acc}{:02x}", b));

        write!(f, "PermanentId(0x{id_str})")
    }
}
