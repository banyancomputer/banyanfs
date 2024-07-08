use futures::{AsyncWrite, AsyncWriteExt};
use std::{
    cmp::PartialEq,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use winnow::{binary::le_u64, Parser};

use crate::codec::{ParserResult, Stream};

/// Vector clocks are used as monotonic clocks for a particular actor or resource within the
/// filesystem and is used for providing strict ordering of events. The internal value is
/// initialized to a random value when a new one is initialized.
///
/// Internally this uses an atomic 64 unsigned integer for the ID, wrapping is an allowed behavior
/// and must be handled by all consumers. We consider a roll over valid if the last known ID was
/// within 262,144 (2^18) ticks of rolling over.

#[derive(Debug, Clone)]
pub struct ClockInner(Arc<AtomicU64>);

impl ClockInner {
    pub fn initialize() -> Self {
        // TODO: make this actually initialize to a random value as the docs above indicate
        Self::from(ClockInnerSnapshot::from(0))
    }

    pub fn to_snapshot(&self) -> ClockInnerSnapshot {
        ClockInnerSnapshot::from(self)
    }
}

impl From<ClockInnerSnapshot> for ClockInner {
    fn from(val: ClockInnerSnapshot) -> Self {
        Self(Arc::new(AtomicU64::new(val.0)))
    }
}

impl PartialEq for ClockInner {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .load(Ordering::Relaxed)
            .eq(&other.0.load(Ordering::Relaxed))
    }
}

const WRAP_THRESHOLD: u64 = 2 ^ 18;

/// A snapshot of a [`VectorClock`] at a specific value
///
/// These are what get stored to record the state of a vector
/// during specific operations
///
/// # Wrapping Behavior
/// These must functionally monotonically increase, but if we overflow
/// the underlying value we will wrap around. This is handled by
/// comparing the values with a threshold to determine if the
/// wrapped value is greater than the non-wrapped value.
/// The threshold is 2^18, or 262,144.
/// [`PartialOrd`] and [`Ord`] are implemented to handle this.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ClockInnerSnapshot(pub(super) u64);

impl ClockInnerSnapshot {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let clock_bytes = self.0.to_le_bytes();

        writer.write_all(&clock_bytes).await?;

        Ok(clock_bytes.len())
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, value) = le_u64.parse_peek(input)?;
        Ok((input, Self(value)))
    }

    pub const fn size() -> usize {
        8
    }
}

impl From<&ClockInner> for ClockInnerSnapshot {
    fn from(value: &ClockInner) -> Self {
        Self(value.0.load(Ordering::Relaxed))
    }
}

impl From<u64> for ClockInnerSnapshot {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl PartialOrd for ClockInnerSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ClockInnerSnapshot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.0 < WRAP_THRESHOLD || other.0 < WRAP_THRESHOLD {
            self.0
                .wrapping_add(WRAP_THRESHOLD)
                .cmp(&other.0.wrapping_add(WRAP_THRESHOLD))
        } else {
            self.0.cmp(&other.0)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use winnow::Partial;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    /// Test PartialOrd implementation
    #[test]
    fn test_partial_ord() {
        // Basic ordering
        assert!(ClockInnerSnapshot(1) < ClockInnerSnapshot(2));
        assert!(ClockInnerSnapshot(2) > ClockInnerSnapshot(1));
        assert!(ClockInnerSnapshot(1) == ClockInnerSnapshot(1));

        // Wrapping
        assert!(ClockInnerSnapshot(u64::MAX) < ClockInnerSnapshot(0));
        assert!(ClockInnerSnapshot(0) > ClockInnerSnapshot(u64::MAX));
        assert!(ClockInnerSnapshot(u64::MAX) < ClockInnerSnapshot(WRAP_THRESHOLD - 1));
        assert!(ClockInnerSnapshot(u64::MAX) > ClockInnerSnapshot(WRAP_THRESHOLD));
    }

    /// Test Ord implementation
    #[test]
    fn test_ord() {
        // Basic ordering
        assert_eq!(
            ClockInnerSnapshot(1).cmp(&ClockInnerSnapshot(2)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            ClockInnerSnapshot(2).cmp(&ClockInnerSnapshot(1)),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            ClockInnerSnapshot(1).cmp(&ClockInnerSnapshot(1)),
            std::cmp::Ordering::Equal
        );

        // Wrapping
        assert_eq!(
            ClockInnerSnapshot(u64::MAX).cmp(&ClockInnerSnapshot(0)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            ClockInnerSnapshot(0).cmp(&ClockInnerSnapshot(u64::MAX)),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            ClockInnerSnapshot(u64::MAX).cmp(&ClockInnerSnapshot(WRAP_THRESHOLD - 1)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            ClockInnerSnapshot(u64::MAX).cmp(&ClockInnerSnapshot(WRAP_THRESHOLD)),
            std::cmp::Ordering::Greater
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_user_agent_roundtrip() {
        let checkpoint = ClockInner::initialize();

        let mut buffer = Vec::with_capacity(ClockInnerSnapshot::size());
        checkpoint
            .to_snapshot()
            .encode(&mut buffer)
            .await
            .expect("encoding success");

        let partial = Partial::new(buffer.as_slice());
        let (remaining, parsed) = ClockInnerSnapshot::parse(partial).expect("round trip");

        let parsed = ClockInner::from(parsed);

        assert!(remaining.is_empty());
        assert_eq!(checkpoint, parsed);
    }
}
