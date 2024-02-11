use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::error::Error as NomError;
use nom::error::ErrorKind;

use crate::codec::AsyncEncodable;

#[derive(Debug, PartialEq)]
pub enum NodeType {
    File,
}

impl NodeType {
    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self, NomError<&[u8]>> {
        let (input, node_type) = take(1u8)(input)?;
        let node_type = node_type[0];

        let parsed_type = match node_type {
            0x00 => Self::File,
            _ => return Err(nom::Err::Error(NomError::new(input, ErrorKind::Tag))),
        };

        Ok((input, parsed_type))
    }
}

#[async_trait]
impl AsyncEncodable for NodeType {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        pos: usize,
    ) -> std::io::Result<usize> {
        let type_byte = match self {
            Self::File => 0x00,
        };

        writer.write_all(&[type_byte]).await?;
        Ok(pos + 1)
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
        let raw_bytes = [0x00];

        let mut encoded = Vec::new();
        node_type.encode(&mut encoded, 0).await.unwrap();
        assert_eq!(raw_bytes, encoded.as_slice());

        let (remaining, parsed) = NodeType::parse(&raw_bytes).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(node_type, parsed);
    }
}
