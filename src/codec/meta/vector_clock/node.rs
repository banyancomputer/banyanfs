use super::clock_inner::{ClockInner, ClockInnerSnapshot};
use crate::codec::{ParserResult, Stream};

use futures::AsyncWrite;

#[derive(Debug, PartialEq, Clone)]
pub struct Node {
    clock: ClockInner,
}

impl Node {
    fn new(clock: ClockInner) -> Self {
        Self { clock }
    }

    pub fn initialize() -> Self {
        Self::new(ClockInner::initialize())
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, clock_snapshot) = NodeSnapshot::parse(input)?;
        Ok((input, Self::new(clock_snapshot.clock.into())))
    }
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        NodeSnapshot::from(self).encode(writer).await
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NodeSnapshot {
    clock: ClockInnerSnapshot,
}

impl NodeSnapshot {
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

impl PartialOrd for NodeSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeSnapshot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.clock.cmp(&other.clock)
    }
}

impl From<&Node> for NodeSnapshot {
    fn from(value: &Node) -> Self {
        Self::new((&value.clock).into())
    }
}
