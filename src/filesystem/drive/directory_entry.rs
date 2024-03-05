use std::sync::Arc;

use async_std::sync::RwLock;

use crate::codec::crypto::SigningKey;
use crate::codec::filesystem::NodeKind;
use crate::codec::PermanentId;
use crate::filesystem::drive::InnerDrive;
use crate::filesystem::nodes::{Node, NodeId, NodeName};

pub struct DirectoryEntry {
    permanent_id: PermanentId,
    name: NodeName,
    kind: NodeKind,

    // these are needed for hydration
    current_key: Arc<SigningKey>,
    cwd_id: NodeId,
    inner: Arc<RwLock<InnerDrive>>,
}

impl DirectoryEntry {
    pub fn name(&self) -> NodeName {
        self.name.clone()
    }

    pub fn kind(&self) -> NodeKind {
        self.kind.clone()
    }

    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }
}
