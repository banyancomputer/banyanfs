use elliptic_curve::rand_core::CryptoRngCore;
use std::collections::HashMap;
use time::OffsetDateTime;

use crate::codec::meta::ActorId;
use crate::filesystem::nodes::{Node, NodeId, NodeKind, PermanentId};

pub(crate) struct NodeBuilder {
    node_id: NodeId,
    parent_id: Option<NodeId>,

    owner_id: ActorId,

    kind: NodeKind,
    metadata: HashMap<String, Vec<u8>>,
}

impl NodeBuilder {
    pub fn directory(node_id: NodeId, owner_id: ActorId) -> Self {
        Self {
            node_id,
            parent_id: None,

            owner_id,

            kind: NodeKind::new_directory(),
            metadata: HashMap::new(),
        }
    }

    pub fn build(self, rng: &mut impl CryptoRngCore) -> Node {
        Node {
            node_id: self.node_id,
            parent_id: self.parent_id,

            owner_id: self.owner_id,
            permanent_id: PermanentId::generate(rng),

            created_at: OffsetDateTime::now_utc(),
            modified_at: OffsetDateTime::now_utc(),

            kind: self.kind,
            metadata: self.metadata,
        }
    }
}
