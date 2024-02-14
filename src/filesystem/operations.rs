use std::path::Path;

use async_trait::async_trait;

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
    #[error("the {0} operation is not supported by this node type")]
    IncompatibleType(&'static str),

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
