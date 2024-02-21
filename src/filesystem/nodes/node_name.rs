use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use futures::{AsyncWrite, AsyncWriteExt};

use crate::codec::ParserResult;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeName(NodeNameInner);

const NAME_TYPE_ROOT_ID: u8 = 0x00;

const NAME_TYPE_NAMED_ID: u8 = 0x01;

impl NodeName {
    pub fn as_str(&self) -> &str {
        match &self.0 {
            NodeNameInner::Root => "{:root:}",
            NodeNameInner::Named(name) => name,
        }
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        match &self.0 {
            NodeNameInner::Root => {
                writer.write_all(&[NAME_TYPE_ROOT_ID]).await?;
                written_bytes += 1;
            }
            NodeNameInner::Named(name) => {
                let name_bytes = name.as_bytes();
                if name_bytes.len() > 255 {
                    return Err(StdError::new(
                        StdErrorKind::InvalidInput,
                        NodeNameError::TooLong(name_bytes.len()),
                    ));
                }

                let name_bytes_length = name_bytes.len() as u8;
                writer
                    .write_all(&[NAME_TYPE_NAMED_ID, name_bytes_length])
                    .await?;
                written_bytes += 2;

                writer.write_all(name_bytes).await?;
                written_bytes += name_bytes.len();
            }
        }

        Ok(written_bytes)
    }

    pub(crate) fn named(name: String) -> Result<Self, NodeNameError> {
        if name.is_empty() {
            return Err(NodeNameError::Empty);
        }

        let byte_length = name.as_bytes().len();
        if byte_length > 255 {
            return Err(NodeNameError::TooLong(byte_length));
        }

        // some reserved names
        match name.as_str() {
            "." | ".." => return Err(NodeNameError::ReservedDirectoryTraversal),
            "{:root:}" => return Err(NodeNameError::ReservedRoot),
            _ => {}
        }

        // todo: extra validation, reserved names and characters etc..

        Ok(Self(NodeNameInner::Named(name)))
    }

    pub fn is_root(&self) -> bool {
        matches!(self.0, NodeNameInner::Root)
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        todo!()
    }

    pub(crate) fn root() -> Self {
        Self(NodeNameInner::Root)
    }
}

impl std::convert::TryFrom<&str> for NodeName {
    type Error = NodeNameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::named(value.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeNameError {
    #[error("name can't be empty")]
    Empty,

    #[error("name can't be '{{:root:}}' as it's reserved in the protocol")]
    ReservedRoot,

    #[error("both '.' nor '..' are directory traversal commands and can not be used as names")]
    ReservedDirectoryTraversal,

    #[error("name can be a maximum of 255 bytes, name was {0} bytes")]
    TooLong(usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum NodeNameInner {
    Root,
    Named(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[ignore]
    async fn test_naming_round_trip() {
        todo!()
    }
}
