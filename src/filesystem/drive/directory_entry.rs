use crate::codec::filesystem::NodeKind;
use crate::codec::PermanentId;
use crate::filesystem::drive::OperationError;
use crate::filesystem::nodes::{Node, NodeName};

/// An immutable view of one of the children of a directory in the filesystem, gets returned by `DirectoryHandle::ls()`
#[derive(Debug)]
pub struct DirectoryEntry {
    permanent_id: PermanentId,

    created_at: i64,
    modified_at: i64,

    name: NodeName,
    kind: NodeKind,

    size: u64,
}

impl DirectoryEntry {
    /// Entry's creation timestamp
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    /// Entry's `NodeKind`
    pub fn kind(&self) -> NodeKind {
        self.kind.clone()
    }

    /// Entry's last modification timestamp
    pub fn modified_at(&self) -> i64 {
        self.modified_at
    }

    /// Entry's Name
    pub fn name(&self) -> NodeName {
        self.name.clone()
    }

    /// Entry's `PermanentId`
    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }

    /// Entry's content size
    /// (i.e. for a File the size of all file content, for a subdirectory the size of the files it and it sub-directories hold)
    pub fn size(&self) -> u64 {
        self.size
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

            size: node.size(),
        })
    }
}
