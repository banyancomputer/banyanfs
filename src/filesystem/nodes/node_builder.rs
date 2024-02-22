use elliptic_curve::rand_core::CryptoRngCore;
use std::collections::HashMap;

use crate::codec::filesystem::NodeKind;
use crate::codec::meta::ActorId;
use crate::filesystem::nodes::{Node, NodeData, NodeId, NodeName, NodeNameError, PermanentId};

pub(crate) struct NodeBuilder {
    id: Option<NodeId>,
    parent_id: Option<NodeId>,

    name: NodeName,
    owner_id: Option<ActorId>,

    kind: NodeKind,
    metadata: HashMap<String, Vec<u8>>,
}

impl NodeBuilder {
    pub fn build(self, rng: &mut impl CryptoRngCore) -> Result<Node, NodeBuilderError> {
        let id = self.id.ok_or(NodeBuilderError::MissingNodeId)?;
        let owner_id = self.owner_id.ok_or(NodeBuilderError::MissingOwner)?;

        // Only the root node is allowed to be without a parent
        if self.parent_id.is_none() && !self.name.is_root() {
            return Err(NodeBuilderError::MissingParent);
        }

        let current_ts = crate::utils::current_time_ms();

        let inner = match self.kind {
            NodeKind::File => NodeData::stub_file(0),
            NodeKind::Directory => NodeData::new_directory(),
            _ => unimplemented!("haven't made it there yet"),
        };

        let new_node = Node {
            id,
            parent_id: self.parent_id,

            cid: None,
            permanent_id: PermanentId::generate(rng),

            name: self.name,
            owner_id,

            created_at: current_ts,
            modified_at: current_ts,

            inner,
            metadata: self.metadata,
        };

        Ok(new_node)
    }

    pub fn directory(name: NodeName) -> Self {
        Self {
            id: None,
            parent_id: None,

            name,
            owner_id: None,

            kind: NodeKind::Directory,
            metadata: HashMap::new(),
        }
    }

    pub fn with_id(mut self, id: NodeId) -> Self {
        self.id = Some(id);
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
            id: None,
            parent_id: None,

            name: NodeName::root(),
            owner_id: None,

            kind: NodeKind::Directory,
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
