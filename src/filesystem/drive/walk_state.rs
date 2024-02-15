use crate::filesystem::nodes::{NodeId, NodeName};

pub(crate) enum WalkState<'a> {
    /// The path was traversed and the final path component was a node
    FoundNode { node_id: NodeId },

    /// While traversing the path, one or more of the path elements
    MissingComponent(NodeId, &'a [&'a str]),

    /// Part of the provided path was not a directory so traversal was stopped. The last valid
    /// directory ID and the remaining path is returned.
    NotTraversable {
        working_directory: NodeId,
        blocking_node: NodeName,
        remaining_path: &'a [&'a str],
    },
}

impl WalkState<'_> {
    pub(crate) fn found(node_id: NodeId) -> Self {
        Self::FoundNode { node_id }
    }
}
