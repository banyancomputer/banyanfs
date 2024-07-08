use super::clock_inner::{ClockInner, ClockInnerSnapshot};
use crate::codec::{ActorId, ParserResult, Stream};

use futures::AsyncWrite;

#[derive(Debug, PartialEq, Clone)]
pub struct Actor {
    id: ActorId,
    clock: ClockInner,
}

impl Actor {
    fn new(id: ActorId, clock: ClockInner) -> Self {
        Self { id, clock }
    }

    pub fn initialize(actor_id: ActorId) -> Self {
        Self::new(actor_id, ClockInner::initialize())
    }

    pub fn as_snapshot(&self) -> ActorSnapshot {
        self.into()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        ActorSnapshot::from(self).encode(writer).await
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, snapshot) = ActorSnapshot::parse(input)?;
        Ok((input, Self::new(snapshot.id, snapshot.clock.into())))
    }
}

impl From<ActorSnapshot> for Actor {
    fn from(value: ActorSnapshot) -> Self {
        Self::new(value.id, value.clock.into())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ActorSnapshot {
    id: ActorId,
    clock: ClockInnerSnapshot,
}

impl ActorSnapshot {
    fn new(id: ActorId, clock: ClockInnerSnapshot) -> Self {
        Self { id, clock }
    }

    pub const fn size() -> usize {
        ActorId::size() + ClockInnerSnapshot::size()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        self.id.encode(writer).await?;
        self.clock.encode(writer).await
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, id) = ActorId::parse(input)?;
        let (input, clock) = ClockInnerSnapshot::parse(input)?;
        Ok((input, Self::new(id, clock)))
    }
}

impl PartialOrd for ActorSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ActorSnapshot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id).then(self.clock.cmp(&other.clock))
    }
}

impl From<&Actor> for ActorSnapshot {
    fn from(value: &Actor) -> Self {
        Self {
            id: value.id,
            clock: (&value.clock).into(),
        }
    }
}
