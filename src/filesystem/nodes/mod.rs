mod directory;
mod file;
mod node_builder;
mod node_type;

pub(crate) use node_builder::NodeBuilder;

pub use directory::Directory;
pub use file::File;
pub use node_type::NodeType;

use std::collections::HashMap;
use time::OffsetDateTime;

use crate::codec::meta::ActorId;

pub(crate) type NodeId = usize;

pub(crate) type PermanentNodeId = [u8; 16];

#[derive(Debug)]
pub struct Node {
    node_id: NodeId,
    parent_id: Option<NodeId>,

    owner_id: ActorId,
    permanent_id: PermanentNodeId,

    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,

    node: NodeType,
    metadata: HashMap<String, Vec<u8>>,
}

impl Node {
    pub fn set_attribute(&mut self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        self.metadata.insert(key, value)
    }

    pub fn owner_id(&self) -> ActorId {
        self.owner_id
    }
}
