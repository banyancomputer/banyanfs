use std::collections::HashMap;

use crate::codec::filesystem::NodeKind;
use crate::codec::PermanentId;
use crate::filesystem::drive::{DirectoryHandle, OperationError};
use crate::filesystem::nodes::{Node, NodeName};

pub struct DirectoryEntry {
    permanent_id: PermanentId,
    name: NodeName,
    kind: NodeKind,

    cwd_handle: DirectoryHandle,
}

impl DirectoryEntry {
    pub fn name(&self) -> NodeName {
        self.name.clone()
    }

    pub fn kind(&self) -> NodeKind {
        self.kind.clone()
    }

    pub async fn metadata(&self) -> Result<HashMap<String, Vec<u8>>, OperationError> {
        tracing::warn!("metadata is not yet being extracted for nodes");
        // note(sstelfox): not ideal to clone all the additional metadata, but it is lazy at least
        // and is sufficient for now.
        Ok(HashMap::new())
    }

    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }
}

impl std::fmt::Debug for DirectoryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectoryEntry")
            .field("permanent_id", &self.permanent_id)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .finish()
    }
}

impl From<(&DirectoryHandle, &Node)> for DirectoryEntry {
    fn from(value: (&DirectoryHandle, &Node)) -> Self {
        Self {
            permanent_id: value.1.permanent_id(),
            name: value.1.name().clone(),
            kind: value.1.kind().clone(),

            cwd_handle: value.0.clone(),
        }
    }
}
