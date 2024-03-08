use crate::codec::filesystem::NodeKind;
use crate::codec::PermanentId;
use crate::filesystem::drive::OperationError;
use crate::filesystem::nodes::{Node, NodeName};

#[derive(Debug)]
pub struct DirectoryEntry {
    permanent_id: PermanentId,

    created_at: i64,
    modified_at: i64,

    name: NodeName,
    kind: NodeKind,
}

impl DirectoryEntry {
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn kind(&self) -> NodeKind {
        self.kind.clone()
    }

    pub fn modified_at(&self) -> i64 {
        self.modified_at
    }

    pub fn name(&self) -> NodeName {
        self.name.clone()
    }

    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }
}

impl TryFrom<&Node> for DirectoryEntry {
    type Error = OperationError;

    fn try_from(node: &Node) -> Result<Self, Self::Error> {
        Ok(Self {
            permanent_id: node.permanent_id(),

            created_at: node.created_at(),
            modified_at: node.modified_at(),

            name: node.name().clone(),
            kind: node.kind().clone(),
        })
    }
}
