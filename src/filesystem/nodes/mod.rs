mod node_builder;
mod node_kind;

pub(crate) use node_builder::{NodeBuilder, NodeBuilderError};

pub use node_kind::NodeKind;

use std::collections::HashMap;
use time::OffsetDateTime;

use crate::codec::meta::{ActorId, PermanentId};

pub(crate) type NodeId = usize;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeName(NodeNameInner);

impl NodeName {
    pub fn as_str(&self) -> &str {
        match &self.0 {
            NodeNameInner::Root => "{:root:}",
            NodeNameInner::Named(name) => name,
        }
    }

    pub(crate) fn named(name: String) -> Result<Self, NodeNameError> {
        if name.is_empty() {
            return Err(NodeNameError::Empty);
        }

        let byte_length = name.as_bytes().len();
        if byte_length > 255 {
            return Err(NodeNameError::TooLong(byte_length));
        }

        // some reserved names
        match name.as_str() {
            "." | ".." => return Err(NodeNameError::ReservedDirectoryTraversal),
            "{:root:}" => return Err(NodeNameError::ReservedRoot),
            _ => {}
        }

        // todo: extra validation, reserved names and characters etc..

        Ok(Self(NodeNameInner::Named(name)))
    }

    pub fn is_root(&self) -> bool {
        matches!(self.0, NodeNameInner::Root)
    }

    pub(crate) fn root() -> Self {
        Self(NodeNameInner::Root)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeNameError {
    #[error("name can't be empty")]
    Empty,

    #[error("name can't be '{{:root:}}' as it's reserved in the protocol")]
    ReservedRoot,

    #[error("both '.' nor '..' are directory traversal commands and can not be used as names")]
    ReservedDirectoryTraversal,

    #[error("name can be a maximum of 255 bytes, name was {0} bytes")]
    TooLong(usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum NodeNameInner {
    Root,
    Named(String),
}

pub struct Node {
    node_id: NodeId,
    parent_id: Option<NodeId>,

    name: NodeName,

    owner_id: ActorId,
    permanent_id: PermanentId,

    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,

    kind: NodeKind,
    metadata: HashMap<String, Vec<u8>>,
}

impl Node {
    pub fn is_directory(&self) -> bool {
        matches!(self.kind, NodeKind::Directory { .. })
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn kind_mut(&mut self) -> &mut NodeKind {
        &mut self.kind
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
