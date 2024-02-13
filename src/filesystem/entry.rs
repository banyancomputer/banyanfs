use elliptic_curve::rand_core::CryptoRngCore;
use rand::Rng;
use std::collections::HashMap;
use time::OffsetDateTime;

use crate::codec::meta::ActorId;
use crate::filesystem::nodes::Node;

pub(crate) type EntryId = usize;

pub(crate) type PermanentEntryId = [u8; 16];

#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) node_id: EntryId,
    pub(crate) parent_id: Option<EntryId>,

    pub(crate) owner_id: ActorId,
    pub(crate) permanent_id: PermanentEntryId,

    pub(crate) created_at: OffsetDateTime,
    pub(crate) modified_at: OffsetDateTime,

    pub(crate) node: Node,
    pub(crate) metadata: HashMap<String, Vec<u8>>,
}

impl Entry {
    pub fn set_attribute(&mut self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        self.metadata.insert(key, value)
    }

    pub fn owner_id(&self) -> ActorId {
        self.owner_id
    }
}
