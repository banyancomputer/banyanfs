use super::clock_inner::{ClockInner, ClockInnerSnapshot};
use crate::codec::{ParserResult, Stream};

use futures::AsyncWrite;

#[derive(Debug, PartialEq, Clone)]
pub struct Filesystem {
    clock: ClockInner,
}

impl Filesystem {
    fn new(clock: ClockInner) -> Self {
        Self { clock }
    }

    pub fn initialize() -> Self {
        Self::new(ClockInner::initialize())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FilesystemSnapshot {
    clock: ClockInnerSnapshot,
}

impl FilesystemSnapshot {
    fn new(clock: ClockInnerSnapshot) -> Self {
        Self { clock }
    }

    pub fn size() -> usize {
        ClockInnerSnapshot::size()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        self.clock.encode(writer).await
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, clock) = ClockInnerSnapshot::parse(input)?;
        Ok((input, Self::new(clock)))
    }
}

impl PartialOrd for FilesystemSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FilesystemSnapshot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.clock.cmp(&other.clock)
    }
}

impl From<&Filesystem> for FilesystemSnapshot {
    fn from(value: &Filesystem) -> Self {
        Self::new((&value.clock).into())
    }
}
