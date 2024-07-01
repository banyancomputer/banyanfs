//! A snapshot of a vector clock at specific value
//! These are what get stored to record the state of a vector
//! during specific operations

use super::{ClockType, VectorClock};
use crate::codec::{ParserResult, Stream};

use futures::{AsyncWrite, AsyncWriteExt};
use std::{marker::PhantomData, sync::atomic::Ordering};
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
#[derive(Debug,  Clone, Copy, PartialEq, Eq)]
pub struct Snapshot<T: ClockType>(pub(super) u64, PhantomData<T>);

// impl<T:ClockType> PartialEq for Snapshot<T> {
//     fn eq(&self, other: &Self) -> bool {
//         self.0 == other.0
//     }

// }

// impl<T:ClockType> Eq for Snapshot<T> {}

impl<T:ClockType> Snapshot<T> {
    fn new(value: u64) -> Self {
        Self(value, PhantomData)
    }

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
        Ok((input, Self(value, PhantomData)))
    }

    pub const fn size() -> usize {
        8
    }
}

impl<T:ClockType> From<&VectorClock<T>> for Snapshot<T> {
    fn from(value: &VectorClock<T>) -> Self {
        Self(value.0.load(Ordering::Relaxed), PhantomData)
    }
}

impl<T:ClockType> From<u64> for Snapshot<T> {
    fn from(value: u64) -> Self {
        Self(value, PhantomData)
    }
}

impl<T:ClockType> PartialOrd for Snapshot<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T:ClockType> Ord for Snapshot<T> {
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
    use crate::codec::meta::vector_clock::FileSystem;

    use super::*;

    /// Test PartialOrd implementation
    #[test]
    fn test_partial_ord() {
        // Basic ordering
        assert!(Snapshot::<FileSystem>::new(1) < Snapshot::new(2));
        assert!(Snapshot::<FileSystem>::new(2) > Snapshot::new(1));
        assert!(Snapshot::<FileSystem>::new(1) == Snapshot::new(1));

        // Wrapping
        assert!(Snapshot::<FileSystem>::new(u64::MAX) < Snapshot::new(0));
        assert!(Snapshot::<FileSystem>::new(0) > Snapshot::new(u64::MAX));
        assert!(Snapshot::<FileSystem>::new(u64::MAX) < Snapshot::new(WRAP_THRESHOLD - 1));
        assert!(Snapshot::<FileSystem>::new(u64::MAX) > Snapshot::new(WRAP_THRESHOLD));
    }

    /// Test Ord implementation
    #[test]
    fn test_ord() {
        // Basic ordering
        assert_eq!(Snapshot::<FileSystem>::new(1).cmp(&Snapshot::new(2)), std::cmp::Ordering::Less);
        assert_eq!(Snapshot::<FileSystem>::new(2).cmp(&Snapshot::new(1)), std::cmp::Ordering::Greater);
        assert_eq!(Snapshot::<FileSystem>::new(1).cmp(&Snapshot::new(1)), std::cmp::Ordering::Equal);

        // Wrapping
        assert_eq!(
            Snapshot::<FileSystem>::new(u64::MAX).cmp(&Snapshot::new(0)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            Snapshot::<FileSystem>::new(0).cmp(&Snapshot::new(u64::MAX)),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            Snapshot::<FileSystem>::new(u64::MAX).cmp(&Snapshot::new(WRAP_THRESHOLD - 1)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            Snapshot::<FileSystem>::new(u64::MAX).cmp(&Snapshot::new(WRAP_THRESHOLD)),
            std::cmp::Ordering::Greater
        );
    }
}
