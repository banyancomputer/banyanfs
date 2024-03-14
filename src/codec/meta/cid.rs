use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;

use crate::codec::ParserResult;

const CID_LENGTH: usize = 32;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Cid([u8; CID_LENGTH]);

impl Cid {
    pub const IDENTITY: Cid = Cid([0u8; CID_LENGTH]);

    pub fn as_bytes(&self) -> &[u8; CID_LENGTH] {
        &self.0
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }

    pub fn as_base64url_multicodec(&self) -> String {
        todo!()
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (remaining, cid_bytes) = take(CID_LENGTH)(input)?;

        let mut bytes = [0u8; CID_LENGTH];
        bytes.copy_from_slice(cid_bytes);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        CID_LENGTH
    }
}

impl std::fmt::Debug for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cid_str = crate::codec::utils::bytes_to_hex_string(&self.0);
        write!(f, "{{0x{cid_str}}}")
    }
}

impl From<[u8; CID_LENGTH]> for Cid {
    fn from(bytes: [u8; CID_LENGTH]) -> Self {
        Self(bytes)
    }
}
