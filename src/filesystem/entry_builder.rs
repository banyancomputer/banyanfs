use elliptic_curve::rand_core::CryptoRngCore;
use rand::Rng;
use std::collections::HashMap;
use time::OffsetDateTime;

use crate::codec::meta::ActorId;
use crate::filesystem::nodes::Node;
use crate::filesystem::{Entry, EntryId, PermanentEntryId};

pub(crate) struct EntryBuilder {
    node_id: EntryId,
    parent_id: Option<EntryId>,

    owner_id: ActorId,

    node: Node,
    metadata: HashMap<String, Vec<u8>>,
}

impl EntryBuilder {
    pub fn build(self, rng: &mut impl CryptoRngCore) -> Entry {
        let permanent_id: PermanentEntryId = rng.gen();

        Entry {
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

    pub fn directory(node_id: EntryId, owner_id: ActorId) -> Self {
        Self {
            node_id,
            parent_id: None,

            owner_id,

            node: Node::Directory(Default::default()),
            metadata: HashMap::new(),
        }
    }
}
