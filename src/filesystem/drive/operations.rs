use std::path::Path;

use async_trait::async_trait;

use crate::codec::meta::PermanentId;
use crate::filesystem::nodes::{NodeId, NodeNameError};

#[async_trait]
pub(crate) trait Deletable {
    async fn rm(&mut self) -> Result<(), OperationError>;
}

#[async_trait]
pub trait Listable {
    async fn ls(&self) -> Result<Vec<String>, OperationError>;
}

#[async_trait]
pub(crate) trait Movable {
    async fn mv(&mut self, path: Path) -> Result<(), OperationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    #[error("encountered a kind of node at a location that really should have been safe: {0}")]
    BadSearch(&'static str),

    #[error("attempted to create a node where one already exists (node {0} in place)")]
    Exists(NodeId),

    #[error("the {0} operation is not supported by this node type")]
    IncompatibleType(&'static str),

    #[error("detected internal violation of assumptions (NID:{0}): {1}")]
    InternalCorruption(NodeId, &'static str),

    #[error("attempted to traversal a path to a parent that doesn't exist")]
    InvalidParentDir,

    #[error("node name was invalid: {0:?}")]
    InvalidName(#[from] NodeNameError),

    #[error("missing permanent id in the filesystem: {0:?}")]
    MissingPermanentId(PermanentId),

    #[error("path attempted to traverse a non-directory node")]
    NotADirectory(NodeId),

    #[error("filesystem entries can only be placed under a directory")]
    ParentMustBeDirectory,

    #[error("path component was too long")]
    PathComponentTooLong,

    #[error("provided path or parent directory was not found")]
    PathNotFound,

    #[error("unable to make use of an empty path")]
    UnexpectedEmptyPath,
}

#[async_trait]
pub(crate) trait Readable {
    async fn read(&self) -> Result<Vec<u8>, OperationError>;
}

#[async_trait]
pub(crate) trait Writable {
    async fn write(&mut self, data: &[u8]) -> Result<(), OperationError>;
}
