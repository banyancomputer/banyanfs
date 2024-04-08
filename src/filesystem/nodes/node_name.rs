use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{binary::le_u8, Parser};

use crate::codec::{ParserResult, Stream};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum NodeName {
    Root,
    Named(String),
}

const NAME_TYPE_ROOT_ID: u8 = 0x00;

const NAME_TYPE_NAMED_ID: u8 = 0x01;

impl NodeName {
    pub fn as_str(&self) -> &str {
        match &self {
            Self::Root => "{:root:}",
            Self::Named(name) => name,
        }
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        match &self {
            Self::Root => {
                writer.write_all(&[NAME_TYPE_ROOT_ID]).await?;
                written_bytes += 1;
            }
            Self::Named(name) => {
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

        if name.contains('/') {
            return Err(NodeNameError::ContainsSlash);
        }

        // todo: extra validation, reserved names and characters etc..

        Ok(Self::Named(name))
    }

    pub fn is_root(&self) -> bool {
        matches!(self, Self::Root)
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, name_type) = le_u8.parse_peek(input)?;

        match name_type {
            NAME_TYPE_ROOT_ID => Ok((input, Self::Root)),
            NAME_TYPE_NAMED_ID => {
                let (input, name_length) = le_u8.parse_peek(input)?;
                let (input, name) = winnow::token::take(name_length as usize).parse_peek(input)?;

                let name = String::from_utf8(name.to_vec()).map_err(|_| {
                    winnow::error::ErrMode::Cut(winnow::error::ParserError::from_error_kind(
                        &input,
                        winnow::error::ErrorKind::Verify,
                    ))
                })?;
                Ok((input, Self::Named(name)))
            }
            _ => {
                let err = winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Verify,
                );
                Err(winnow::error::ErrMode::Cut(err))
            }
        }
    }

    pub(crate) fn root() -> Self {
        Self::Root
    }

    pub(crate) fn size(&self) -> usize {
        match self {
            Self::Root => 1,
            Self::Named(name) => 2 + name.len(),
        }
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
    #[error("name can't contain slashes")]
    ContainsSlash,

    #[error("name can't be empty")]
    Empty,

    #[error("name can't be '{{:root:}}' as it's reserved in the protocol")]
    ReservedRoot,

    #[error("both '.' nor '..' are directory traversal commands and can not be used as names")]
    ReservedDirectoryTraversal,

    #[error("name can be a maximum of 255 bytes, name was {0} bytes")]
    TooLong(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_root_round_trip() {
        let original = NodeName::root();

        let mut buffer = Vec::new();
        original.encode(&mut buffer).await.unwrap();
        assert_eq!(buffer, &[0x00]);

        let (remaining, parsed) = NodeName::parse(Stream::new(&buffer)).unwrap();
        let remaining: Vec<u8> = remaining.to_vec();
        assert_eq!(Vec::<u8>::new(), remaining);
        assert_eq!(original, parsed);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_named_round_trip() {
        let original = NodeName::named("hello".to_string()).unwrap();

        let mut buffer = Vec::new();
        original.encode(&mut buffer).await.unwrap();
        assert_eq!(buffer, &[0x01, 0x05, b'h', b'e', b'l', b'l', b'o']);

        let (remaining, parsed) = NodeName::parse(Stream::new(&buffer)).unwrap();
        let remaining: Vec<u8> = remaining.to_vec();
        assert_eq!(Vec::<u8>::new(), remaining);
        assert_eq!(original, parsed);
    }
}
