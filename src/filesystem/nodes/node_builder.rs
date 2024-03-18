use std::collections::HashMap;

use elliptic_curve::rand_core::CryptoRngCore;

use crate::codec::filesystem::NodeKind;
use crate::codec::meta::{ActorId, VectorClock};
use crate::filesystem::nodes::{
    CidCache, Node, NodeData, NodeId, NodeName, NodeNameError, PermanentId,
};

pub(crate) struct NodeBuilder {
    id: Option<NodeId>,
    parent_id: Option<PermanentId>,

    name: NodeName,
    owner_id: Option<ActorId>,
    size_hint: Option<u64>,

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
            NodeKind::File => NodeData::stub_file(self.size_hint.unwrap_or(0)),
            NodeKind::Directory => NodeData::new_directory(),
            _ => unimplemented!("haven't made it there yet"),
        };

        let vector_clock = VectorClock::initialize();

        let new_node = Node {
            id,
            parent_id: self.parent_id,
            permanent_id: PermanentId::generate(rng),
            owner_id,

            cid: CidCache::empty(),
            vector_clock,

            name: self.name,

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
            size_hint: None,

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

    pub fn with_parent(mut self, parent_id: PermanentId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_size_hint(mut self, size_hint: u64) -> Self {
        self.size_hint = Some(size_hint);
        self
    }

    pub(crate) fn root() -> Self {
        Self::directory(NodeName::root())
    }

    pub(crate) fn file(name: NodeName) -> Self {
        Self {
            id: None,
            parent_id: None,

            name,
            owner_id: None,
            size_hint: None,

            kind: NodeKind::File,
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
