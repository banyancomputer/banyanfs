use futures::{AsyncWrite, AsyncWriteExt};
use winnow::bytes::take;

use crate::codec::{ParserResult, Stream};

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    File,
    AssociatedData,
    Directory,
    InternalLink,
    NativeMount,
    Unknown(u8),
}

impl NodeKind {
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
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

    pub fn parse(input: Stream) -> ParserResult<Self> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip_file() {
        let node_type = NodeKind::File;
        let source_bytes = [0x00];

        let (remaining, parsed) = NodeKind::parse(Stream::new(&source_bytes)).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(node_type, parsed);

        let mut encoded = Vec::new();
        let size = node_type.encode(&mut encoded).await.unwrap();

        assert_eq!(source_bytes.len(), size);
        assert_eq!(source_bytes, encoded.as_slice());
    }
}
