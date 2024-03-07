use std::collections::HashMap;

use crate::codec::filesystem::NodeKind;
use crate::codec::PermanentId;
use crate::filesystem::drive::OperationError;
use crate::filesystem::nodes::{Node, NodeId, NodeName};

pub struct DirectoryEntry {
    node_id: NodeId,
    permanent_id: PermanentId,

    name: NodeName,
    kind: NodeKind,

    metadata: HashMap<String, Vec<u8>>,
}

impl DirectoryEntry {
    pub fn name(&self) -> NodeName {
        self.name.clone()
    }

    pub fn kind(&self) -> NodeKind {
        self.kind.clone()
    }

    pub fn metadata(&self) -> &HashMap<String, Vec<u8>> {
        &self.metadata
    }

    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }

    pub(crate) async fn try_from(node: &Node) -> Result<Self, OperationError> {
        let metadata = node.metadata().clone();

        // todo(sstelfox): need to redo the metadata to support multiple types of metadata, really
        // just need to i64, String, and Vec<u8> for now. Will need to merge in created_at and the
        // other "mandatory" metadata fields.

        Ok(Self {
            node_id: node.id(),
            permanent_id: node.permanent_id(),

            name: node.name().clone(),
            kind: node.kind().clone(),

            metadata,
        })
    }
}

impl std::fmt::Debug for DirectoryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectoryEntry")
            .field("name", &self.name)
            .field("permanent_id", &self.permanent_id)
            .field("kind", &self.kind)
            .finish()
    }
}
