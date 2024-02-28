use crate::codec::filesystem::NodeKind;
use crate::codec::PermanentId;
use crate::filesystem::nodes::{Node, NodeName};

#[derive(Debug)]
pub struct DirectoryEntry {
    permanent_id: PermanentId,
    name: NodeName,
    kind: NodeKind,
}

impl From<Node> for DirectoryEntry {
    fn from(node: Node) -> Self {
        Self {
            permanent_id: node.permanent_id(),
            name: node.name(),
            kind: node.kind(),
        }
    }
}
