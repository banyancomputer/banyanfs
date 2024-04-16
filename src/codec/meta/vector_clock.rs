use std::sync::Arc;

use async_std::sync::RwLock;
use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{binary::le_u64, Parser};

use crate::codec::{ParserResult, Stream};

#[derive(Clone, Debug)]
pub struct VectorClock(Arc<RwLock<u64>>);

impl VectorClock {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let readable_clock = self.0.read().await;
        let clock_bytes = readable_clock.to_le_bytes();
        writer.write_all(&clock_bytes).await?;
        Ok(clock_bytes.len())
    }

    pub fn initialize() -> Self {
        Self::from(0)
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, value) = le_u64.parse_peek(input)?;
        Ok((input, Self::from(value)))
    }

    pub const fn size() -> usize {
        8
    }
}

impl From<u64> for VectorClock {
    fn from(val: u64) -> Self {
        Self(Arc::new(RwLock::new(val)))
    }
}
