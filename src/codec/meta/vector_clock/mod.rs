use std::cmp::PartialEq;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::AsyncWrite;

use crate::codec::{ParserResult, Stream};

mod snapshot;
pub use snapshot::Snapshot as VectorClockSnapshot;

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
        VectorClockSnapshot::from(self).encode(writer).await
    }

    pub fn initialize() -> Self {
        // TODO: make this actually initialize to a random value as the docs above indicate
        Self::from(VectorClockSnapshot::from(0))
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, value) = VectorClockSnapshot::parse(input)?;
        Ok((input, Self::from(value)))
    }

    pub const fn size() -> usize {
        8
    }
}

impl From<VectorClockSnapshot> for VectorClock {
    fn from(val: VectorClockSnapshot) -> Self {
        Self(Arc::new(AtomicU64::new(val.0)))
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
