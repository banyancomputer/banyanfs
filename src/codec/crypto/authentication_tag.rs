use std::ops::Deref;

use chacha20poly1305::Tag as ChaChaTag;
use futures::{AsyncWrite, AsyncWriteExt};
use winnow::bytes::streaming::take;

use crate::codec::ParserResult;

const TAG_LENGTH: usize = 16;

#[derive(Clone, Debug)]
pub struct AuthenticationTag([u8; TAG_LENGTH]);

impl AuthenticationTag {
    pub fn as_bytes(&self) -> &[u8; TAG_LENGTH] {
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
        let (remaining, slice) = take(TAG_LENGTH)(input)?;

        let mut bytes = [0u8; TAG_LENGTH];
        bytes.copy_from_slice(slice);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        TAG_LENGTH
    }
}

impl Deref for AuthenticationTag {
    type Target = ChaChaTag;

    fn deref(&self) -> &Self::Target {
        ChaChaTag::from_slice(&self.0)
    }
}

impl From<[u8; TAG_LENGTH]> for AuthenticationTag {
    fn from(bytes: [u8; TAG_LENGTH]) -> Self {
        Self(bytes)
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_authentication_tag_parsing() {
        let mut rng = rand::thread_rng();
        let input: [u8; TAG_LENGTH + 4] = rng.gen();
        let (remaining, tag) = AuthenticationTag::parse(&input).unwrap();

        assert_eq!(remaining, &input[TAG_LENGTH..]);
        assert_eq!(tag.as_bytes(), &input[..TAG_LENGTH]);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_authentication_tag_parsing_stream_too_short() {
        let input = [0u8; TAG_LENGTH - 1];
        let result = AuthenticationTag::parse(&input);
        assert!(matches!(result, Err(winnow::error::ErrMode::Incomplete(_))));
    }
}
