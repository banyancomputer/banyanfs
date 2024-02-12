use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::error::Error as NomError;

use crate::codec::AsyncEncodable;

#[derive(Debug, PartialEq)]
pub enum NodeType {
    File,
    AssociatedData,
    Directory,
    InternalLink,
    NativeMount,
    Unknown(u8),
}

impl NodeType {
    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self, NomError<&[u8]>> {
        let (input, node_type) = take(1u8)(input)?;
        let node_type = node_type[0];

        let parsed_type = match node_type {
            0x00 => Self::File,
            0x01 => Self::AssociatedData,
            0x02 => Self::Directory,
            0x03 => Self::InternalLink,
            0x04 => Self::NativeMount,

            num => Self::Unknown(num),
        };

        Ok((input, parsed_type))
    }
}

#[async_trait]
impl AsyncEncodable for NodeType {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let type_byte = match self {
            Self::File => 0x00,
            Self::AssociatedData => 0x01,
            Self::Directory => 0x02,
            Self::InternalLink => 0x03,
            Self::NativeMount => 0x04,
            Self::Unknown(num) => *num,
        };

        writer.write_all(&[type_byte]).await?;

        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip_file() {
        let node_type = NodeType::File;
        let source_bytes = [0x00];

        let (remaining, parsed) = NodeType::parse(&source_bytes).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(node_type, parsed);

        let mut encoded = Vec::new();
        let size = node_type.encode(&mut encoded).await.unwrap();

        assert_eq!(source_bytes.len(), size);
        assert_eq!(source_bytes, encoded.as_slice());
    }
}
