use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;

use crate::codec::AsyncEncodable;

const CID_LENGTH: usize = 32;

#[derive(Clone, Debug)]
pub struct Cid([u8; CID_LENGTH]);

impl Cid {
    pub fn as_bytes(&self) -> &[u8; CID_LENGTH] {
        &self.0
    }

    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, cid_bytes) = take(CID_LENGTH)(input)?;

        let mut bytes = [0u8; CID_LENGTH];
        bytes.copy_from_slice(cid_bytes);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        CID_LENGTH
    }
}

impl From<[u8; CID_LENGTH]> for Cid {
    fn from(bytes: [u8; CID_LENGTH]) -> Self {
        Self(bytes)
    }
}

#[async_trait]
impl AsyncEncodable for Cid {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }
}
