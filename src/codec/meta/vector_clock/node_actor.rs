use super::{actor::ActorSnapshot, node::NodeSnapshot};
use crate::codec::{ParserResult, Stream};

use futures::AsyncWrite;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NodeActorSnapshot(NodeSnapshot, ActorSnapshot);

impl NodeActorSnapshot {
    pub fn new(node: NodeSnapshot, actor: ActorSnapshot) -> Self {
        Self(node, actor)
    }

    pub fn size() -> usize {
        NodeSnapshot::size() + ActorSnapshot::size()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        self.0.encode(writer).await?;
        self.1.encode(writer).await
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, node) = NodeSnapshot::parse(input)?;
        let (input, actor) = ActorSnapshot::parse(input)?;
        Ok((input, Self::new(node, actor)))
    }
}

impl PartialOrd for NodeActorSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeActorSnapshot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0).then(self.1.cmp(&other.1))
    }
}

impl<N: Into<NodeSnapshot>, A: Into<ActorSnapshot>> From<(N, A)> for NodeActorSnapshot {
    fn from(value: (N, A)) -> Self {
        let (node, actor) = value;
        Self::new(node.into(), actor.into())
    }
}
