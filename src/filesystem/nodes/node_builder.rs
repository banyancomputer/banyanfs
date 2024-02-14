use elliptic_curve::rand_core::CryptoRngCore;
use rand::Rng;
use std::collections::HashMap;
use time::OffsetDateTime;

use crate::codec::meta::ActorId;
use crate::filesystem::nodes::{Node, NodeId, NodeType, PermanentNodeId};

pub(crate) struct NodeBuilder {
    node_id: NodeId,
    parent_id: Option<NodeId>,

    owner_id: ActorId,

    node: NodeType,
    metadata: HashMap<String, Vec<u8>>,
}

impl NodeBuilder {
    pub fn build(self, rng: &mut impl CryptoRngCore) -> Node {
        let permanent_id: PermanentNodeId = rng.gen();

        Node {
            node_id: self.node_id,
            parent_id: self.parent_id,

            owner_id: self.owner_id,
            permanent_id,

            created_at: OffsetDateTime::now_utc(),
            modified_at: OffsetDateTime::now_utc(),

            node: self.node,
            metadata: self.metadata,
        }
    }

    pub fn directory(node_id: NodeId, owner_id: ActorId) -> Self {
        Self {
            node_id,
            parent_id: None,

            owner_id,

            node: NodeType::Directory(Default::default()),
            metadata: HashMap::new(),
        }
    }
}
