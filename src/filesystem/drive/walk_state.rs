use crate::filesystem::nodes::{NodeId, NodeName};

pub(crate) enum WalkState<'a> {
    /// The path was traversed and the final path component was a node
    FoundNode { node_id: NodeId },

    /// While traversing the path, one or more of the path elements wasn't found, this may include
    /// the final element which may be desirable for cases such as the recursive creatio of
    /// directories, generally its better to look up the parent directory and using the found path
    /// instead of handling the failure case on the final element.
    MissingComponent {
        working_directory_id: NodeId,
        missing_name: NodeName,
        remaining_path: &'a [&'a str],
    },
}

impl WalkState<'_> {
    pub(crate) fn found(node_id: NodeId) -> Self {
        Self::FoundNode { node_id }
    }
}
