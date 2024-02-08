use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::sequence::tuple;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::codec::AsyncEncodable;

const CID_LENGTH: usize = 32;

pub(crate) struct Cid([u8; CID_LENGTH]);

impl Cid {
    pub fn as_bytes(&self) -> &[u8; CID_LENGTH] {
        &self.0
    }

    pub(crate) fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, cid_bytes) = take(CID_LENGTH)(input)?;

        let mut bytes = [0u8; CID_LENGTH];
        bytes.copy_from_slice(cid_bytes);

        Ok((remaining, Self(bytes)))
    }
}

#[async_trait::async_trait]
impl AsyncEncodable for Cid {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> tokio::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(start_pos + self.0.len())
    }
}
