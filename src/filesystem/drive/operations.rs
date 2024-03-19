use crate::codec::meta::PermanentId;
use crate::codec::Cid;
use crate::filesystem::nodes::{NodeBuilderError, NodeError, NodeId, NodeNameError};
use crate::filesystem::{DataStoreError, FileContentError};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OperationError {
    #[error("current user doesn't have the correct key to read or write to the drive")]
    AccessDenied,

    #[error("block was found but wasn't valid: {0:?}")]
    BlockCorrupted(Cid),

    #[error("block with CID was not found in the data store: {0:?}")]
    BlockUnavailable(Cid),

    #[error("creation of a node failed: {0}")]
    CreationFailed(#[from] NodeBuilderError),

    #[error("data store operation failed: {0}")]
    DataStore(#[from] DataStoreError),

    #[error("attempted to create a node where one already exists (node {0} in place)")]
    Exists(NodeId),

    #[error("error working with file content: {0}")]
    FileContentError(#[from] FileContentError),

    #[error("detected internal violation of assumptions (NID:{0}): {1}")]
    InternalCorruption(NodeId, &'static str),

    #[error("node name was invalid: {0:?}")]
    InvalidName(#[from] NodeNameError),

    #[error("missing permanent id in the filesystem: {0:?}")]
    MissingPermanentId(PermanentId),

    #[error("node operation failed: {0}")]
    NodeFailure(#[from] NodeError),

    #[error("the requested content hasn't been uploaded and recorded yet")]
    NotAvailable,

    #[error("path attempted to traverse a leaf node")]
    NotTraversable,

    #[error("attempted to read from a node that contains no data")]
    NotReadable,

    #[error("Node({0:?}) was orphaned in filesystem and is unsafe to remove")]
    OrphanNode(PermanentId),

    #[error("I needed a temporary catch all error: {0}")]
    Other(&'static str),

    #[error("filesystem entries can only be placed under a directory")]
    ParentMustBeDirectory,

    #[error("provided path or parent directory was not found")]
    PathNotFound,

    #[error("attempted recursion too deep to process")]
    PathTooDeep,

    #[error("unable to make use of an empty path")]
    UnexpectedEmptyPath,
}
