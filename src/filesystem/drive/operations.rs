use crate::codec::meta::PermanentId;
use crate::filesystem::nodes::{NodeBuilderError, NodeError, NodeId, NodeNameError};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OperationError {
    #[error("creation of a node failed: {0}")]
    CreationFailed(#[from] NodeBuilderError),

    #[error("attempted to create a node where one already exists (node {0} in place)")]
    Exists(NodeId),

    #[error("detected internal violation of assumptions (NID:{0}): {1}")]
    InternalCorruption(NodeId, &'static str),

    #[error("node name was invalid: {0:?}")]
    InvalidName(#[from] NodeNameError),

    #[error("missing permanent id in the filesystem: {0:?}")]
    MissingPermanentId(PermanentId),

    #[error("node operation failed: {0}")]
    NodeFailure(#[from] NodeError),

    #[error("path attempted to traverse a non-directory node")]
    NotADirectory,

    #[error("attempted to read from a node that contains no data")]
    NotReadable,

    #[error("Node({0:?}) was orphaned in filesystem and is unsafe to remove")]
    OrphanNode(PermanentId),

    #[error("filesystem entries can only be placed under a directory")]
    ParentMustBeDirectory,

    #[error("provided path or parent directory was not found")]
    PathNotFound,

    #[error("attempted recursion too deep to process")]
    PathTooDeep,

    #[error("unable to make use of an empty path")]
    UnexpectedEmptyPath,
}
