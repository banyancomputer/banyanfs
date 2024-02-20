mod node_builder;
mod node_kind;
mod node_name;

pub(crate) use node_builder::{NodeBuilder, NodeBuilderError};

pub use node_kind::NodeKind;
pub use node_name::{NodeName, NodeNameError};

use std::collections::HashMap;

use time::OffsetDateTime;

use crate::codec::meta::{ActorId, PermanentId};

pub(crate) type NodeId = usize;

pub struct Node {
    id: NodeId,
    parent_id: Option<NodeId>,

    pub(crate) permanent_id: PermanentId,
    pub(crate) owner_id: ActorId,

    pub(crate) created_at: OffsetDateTime,
    pub(crate) modified_at: OffsetDateTime,

    pub(crate) name: NodeName,
    pub(crate) metadata: HashMap<String, Vec<u8>>,
    pub(crate) kind: NodeKind,
}

impl Node {
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn is_directory(&self) -> bool {
        matches!(self.kind, NodeKind::Directory { .. })
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn kind_mut(&mut self) -> &mut NodeKind {
        &mut self.kind
    }

    pub fn modified_at(&self) -> OffsetDateTime {
        self.modified_at
    }

    pub fn name(&self) -> NodeName {
        self.name.clone()
    }

    pub fn owner_id(&self) -> ActorId {
        self.owner_id
    }

    pub fn parent_id(&self) -> Option<NodeId> {
        self.parent_id
    }

    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
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
                .field(&self.id)
                .field(&self.owner_id)
                .field(&self.permanent_id)
                .field(&self.name)
                .finish(),
            NodeKind::File { .. } => f
                .debug_tuple("NodeFile")
                .field(&self.id)
                .field(&self.owner_id)
                .field(&self.permanent_id)
                .field(&self.name)
                .finish(),
        }
    }
}
