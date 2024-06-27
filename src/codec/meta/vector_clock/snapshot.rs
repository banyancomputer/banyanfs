//! A snapshot of a vector clock at specific value
//! These are what get stored to record the state of a vector
//! during specific operations

use super::VectorClock;
use crate::codec::{ParserResult, Stream};

use futures::{AsyncWrite, AsyncWriteExt};
use std::sync::atomic::Ordering;
use winnow::{binary::le_u64, Parser};

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
pub struct Snapshot(pub(super) u64);

impl Snapshot {
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

impl From<&VectorClock> for Snapshot {
    fn from(value: &VectorClock) -> Self {
        Self(value.0.load(Ordering::Relaxed))
    }
}

impl From<u64> for Snapshot {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl PartialOrd for Snapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.0 < WRAP_THRESHOLD || other.0 < WRAP_THRESHOLD {
            self.0
                .wrapping_add(WRAP_THRESHOLD)
                .partial_cmp(&other.0.wrapping_add(WRAP_THRESHOLD))
        } else {
            self.0.partial_cmp(&other.0)
        }
    }
}

impl Ord for Snapshot {
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

    /// Test PartialOrd implementation
    #[test]
    fn test_partial_ord() {
        // Basic ordering
        assert!(Snapshot(1) < Snapshot(2));
        assert!(Snapshot(2) > Snapshot(1));
        assert!(Snapshot(1) == Snapshot(1));

        // Wrapping
        assert!(Snapshot(u64::MAX) < Snapshot(0));
        assert!(Snapshot(0) > Snapshot(u64::MAX));
        assert!(Snapshot(u64::MAX) < Snapshot(WRAP_THRESHOLD - 1));
        assert!(Snapshot(u64::MAX) > Snapshot(WRAP_THRESHOLD));
    }

    /// Test Ord implementation
    #[test]
    fn test_ord() {
        // Basic ordering
        assert_eq!(Snapshot(1).cmp(&Snapshot(2)), std::cmp::Ordering::Less);
        assert_eq!(Snapshot(2).cmp(&Snapshot(1)), std::cmp::Ordering::Greater);
        assert_eq!(Snapshot(1).cmp(&Snapshot(1)), std::cmp::Ordering::Equal);

        // Wrapping
        assert_eq!(
            Snapshot(u64::MAX).cmp(&Snapshot(0)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            Snapshot(0).cmp(&Snapshot(u64::MAX)),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            Snapshot(u64::MAX).cmp(&Snapshot(WRAP_THRESHOLD - 1)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            Snapshot(u64::MAX).cmp(&Snapshot(WRAP_THRESHOLD)),
            std::cmp::Ordering::Greater
        );
    }
}
