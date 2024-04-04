use ecdsa::signature::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use winnow::bytes::streaming::take;
use rand::Rng;

use crate::codec::ParserResult;

const PERMANENT_ID_SIZE: usize = 8;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PermanentId([u8; PERMANENT_ID_SIZE]);

impl PermanentId {
    pub fn as_bytes(&self) -> &[u8; PERMANENT_ID_SIZE] {
        &self.0
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        Self(rng.gen())
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (remaining, id_bytes) = take(PERMANENT_ID_SIZE)(input)?;

        let mut bytes = [0u8; PERMANENT_ID_SIZE];
        bytes.copy_from_slice(id_bytes);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        PERMANENT_ID_SIZE
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
