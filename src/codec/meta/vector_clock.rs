use std::cmp::PartialEq;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{binary::le_u64, Parser};

use crate::codec::{ParserResult, Stream};

/// VectorClocks are used as monotonic clocks for a particular actor or resource within the
/// filesystem and is used for providing strict ordering of events. The internal value is
/// initialized to a random value when a new one is initialized.
///
/// Internally this uses an atomic 64 unsigned integer for the ID, wrapping is an allowed behavior
/// and must be handled by all consumers. We consider a roll over valid if the last known ID was
/// within 262,144 (2^18) ticks of rolling over.
#[derive(Clone, Debug)]
pub struct VectorClock(Arc<AtomicU64>);

impl VectorClock {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let current = self.0.load(Ordering::Relaxed);
        let clock_bytes = current.to_le_bytes();

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
        Self(Arc::new(AtomicU64::new(val)))
    }
}

impl PartialEq for VectorClock {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .load(Ordering::Relaxed)
            .eq(&other.0.load(Ordering::Relaxed))
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
        let checkpoint = VectorClock::initialize();

        let mut buffer = Vec::with_capacity(VectorClock::size());
        checkpoint
            .encode(&mut buffer)
            .await
            .expect("encoding success");

        let partial = Partial::new(buffer.as_slice());
        let (remaining, parsed) = VectorClock::parse(partial).expect("round trip");

        assert!(remaining.is_empty());
        assert_eq!(checkpoint, parsed);
    }
}
