mod node_builder;
mod node_kind;

pub(crate) use node_builder::NodeBuilder;

pub use node_kind::NodeKind;

use std::collections::HashMap;
use time::OffsetDateTime;

use crate::codec::meta::ActorId;

pub(crate) type NodeId = usize;

pub(crate) type PermanentNodeId = [u8; 16];

pub struct Node {
    node_id: NodeId,
    parent_id: Option<NodeId>,

    owner_id: ActorId,
    permanent_id: PermanentNodeId,

    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,

    kind: NodeKind,
    metadata: HashMap<String, Vec<u8>>,
}

impl Node {
    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn owner_id(&self) -> ActorId {
        self.owner_id
    }

    pub fn set_attribute(&mut self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        self.metadata.insert(key, value)
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            NodeKind::Directory { .. } => f
                .debug_tuple("NodeDirectory")
                .field(&self.node_id)
                .field(&self.owner_id)
                .field(&self.permanent_id)
                .finish(),
            NodeKind::File { .. } => f
                .debug_tuple("NodeFile")
                .field(&self.node_id)
                .field(&self.owner_id)
                .field(&self.permanent_id)
                .finish(),
        }
    }
}
