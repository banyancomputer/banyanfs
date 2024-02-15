use elliptic_curve::rand_core::CryptoRngCore;
use std::collections::HashMap;
use time::OffsetDateTime;

use crate::codec::meta::ActorId;
use crate::filesystem::nodes::{Node, NodeId, NodeKind, NodeName, NodeNameError, PermanentId};

pub(crate) struct NodeBuilder {
    node_id: Option<NodeId>,
    parent_id: Option<NodeId>,

    name: NodeName,
    owner_id: Option<ActorId>,

    kind: NodeKind,
    metadata: HashMap<String, Vec<u8>>,
}

impl NodeBuilder {
    pub fn build(self, rng: &mut impl CryptoRngCore) -> Result<Node, NodeBuilderError> {
        let node_id = self.node_id.ok_or(NodeBuilderError::MissingNodeId)?;
        let owner_id = self.owner_id.ok_or(NodeBuilderError::MissingOwner)?;

        // Only the root node is allowed to be without a parent
        if self.parent_id.is_none() && !self.name.is_root() {
            return Err(NodeBuilderError::MissingParent);
        }

        let new_node = Node {
            node_id,
            parent_id: self.parent_id,

            name: self.name,
            owner_id,
            permanent_id: PermanentId::generate(rng),

            created_at: OffsetDateTime::now_utc(),
            modified_at: OffsetDateTime::now_utc(),

            kind: self.kind,
            metadata: self.metadata,
        };

        Ok(new_node)
    }

    pub fn directory(name: String) -> Result<Self, NodeBuilderError> {
        let name = NodeName::named(name)?;

        let dir_node = Self {
            node_id: None,
            parent_id: None,

            name,
            owner_id: None,

            kind: NodeKind::new_directory(),
            metadata: HashMap::new(),
        };

        Ok(dir_node)
    }

    pub fn with_node_id(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    pub fn with_owner(mut self, owner_id: ActorId) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    pub fn with_parent(mut self, parent_id: NodeId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub(crate) fn root() -> Self {
        Self {
            node_id: None,
            parent_id: None,

            name: NodeName::root(),
            owner_id: None,

            kind: NodeKind::new_directory(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeBuilderError {
    #[error("node name isn't valid: {0}")]
    InvalidNodeName(#[from] NodeNameError),

    #[error("internal node id must be set before a node can be created")]
    MissingNodeId,

    #[error("unparented nodes are not allowed to exist")]
    MissingParent,

    #[error("all nodes must have an owner")]
    MissingOwner,
}
