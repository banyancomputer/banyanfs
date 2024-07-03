use super::{actor::ActorSnapshot, filesystem::FilesystemSnapshot, Actor, Filesystem};
use crate::codec::{ParserResult, Stream};

use futures::AsyncWrite;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FilesystemActorSnapshot(FilesystemSnapshot, ActorSnapshot);

impl FilesystemActorSnapshot {
    pub fn new(filesystem: FilesystemSnapshot, actor: ActorSnapshot) -> Self {
        Self(filesystem, actor)
    }

    pub const fn size() -> usize {
        FilesystemSnapshot::size() + ActorSnapshot::size()
    }

    pub fn reanimate(self) -> (Filesystem, Actor) {
        (Filesystem::from(self.0), Actor::from(self.1))
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        self.0.encode(writer).await?;
        self.1.encode(writer).await
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, filesystem) = FilesystemSnapshot::parse(input)?;
        let (input, actor) = ActorSnapshot::parse(input)?;
        Ok((input, Self::new(filesystem, actor)))
    }

    pub fn filesystem(&self) -> FilesystemSnapshot {
        self.0
    }

    pub fn actor(&self) -> ActorSnapshot {
        self.1
    }
}

impl PartialOrd for FilesystemActorSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FilesystemActorSnapshot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0).then(self.1.cmp(&other.1))
    }
}

impl<F: Into<FilesystemSnapshot>, A: Into<ActorSnapshot>> From<(F, A)> for FilesystemActorSnapshot {
    fn from(value: (F, A)) -> Self {
        let (filesystem, actor) = value;
        Self::new(filesystem.into(), actor.into())
    }
}
