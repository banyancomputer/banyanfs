use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;

use crate::codec::ParserResult;

const CID_LENGTH: usize = 32;

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cid([u8; CID_LENGTH]);

impl Cid {
    pub const IDENTITY: Cid = Cid([0u8; CID_LENGTH]);

    pub fn as_base64url_multicodec(&self) -> String {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let mut inner_bytes = Vec::with_capacity(CID_LENGTH + 1);

        // raw inner data: cid version 1, raw multicodec 0x55, blake3 multihash 0x1e, len of 32
        inner_bytes.extend_from_slice(&[0x01, 0x55, 0x1e, 0x20]);

        // the hash itself
        inner_bytes.extend_from_slice(&self.0);

        let encoded = URL_SAFE_NO_PAD.encode(&inner_bytes);

        // base code identifier for base64url is 'u'
        format!("u{}", encoded)
    }

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
        write!(f, "{}", self.as_base64url_multicodec())
    }
}

impl From<[u8; CID_LENGTH]> for Cid {
    fn from(bytes: [u8; CID_LENGTH]) -> Self {
        Self(bytes)
    }
}
