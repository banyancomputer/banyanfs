use async_trait::async_trait;
use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use uuid::{NoContext, Timestamp, Uuid};

use crate::codec::{AsyncEncodable, ParserResult};

const ID_LENGTH: usize = 16;

#[derive(Clone, Copy, PartialEq)]
pub struct FilesystemId([u8; ID_LENGTH]);

impl FilesystemId {
    pub fn generate(_rng: &mut impl CryptoRngCore) -> Self {
        // todo: this needs to use the provided rng to generate
        let ts = Timestamp::now(NoContext);
        let uuid = Uuid::new_v7(ts);
        Self(uuid.to_bytes_le())
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (remaining, id_bytes) = take(ID_LENGTH)(input)?;

        // All zeros and all ones are disallowed, this isn't actually harmful though so we'll only
        // perform this check in strict mode.
        if cfg!(feature = "strict")
            && (id_bytes.iter().all(|&b| b == 0x00) || id_bytes.iter().all(|&b| b == 0xff))
        {
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        // todo(sstelfox): parse into an actually UUID, validate the version, probably store the
        // UUID instead of the bytes.

        let mut bytes = [0u8; ID_LENGTH];
        bytes.copy_from_slice(id_bytes);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        ID_LENGTH
    }
}

impl From<[u8; ID_LENGTH]> for FilesystemId {
    fn from(bytes: [u8; ID_LENGTH]) -> Self {
        Self(bytes)
    }
}

#[async_trait]
impl AsyncEncodable for FilesystemId {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }
}

impl std::fmt::Debug for FilesystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let filesystem_id_str: String = self
            .0
            .iter()
            .fold(String::new(), |acc, &b| format!("{acc}{:02x}", b));

        write!(f, "{{0x{filesystem_id_str}}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::Rng;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip() {
        let mut rng = crate::utils::crypto_rng();

        let raw_id: [u8; ID_LENGTH] = rng.gen();
        let filesystem_id = FilesystemId::from(raw_id);

        let mut encoded = Vec::new();
        filesystem_id.encode(&mut encoded).await.unwrap();
        assert_eq!(raw_id, encoded.as_slice());

        let (remaining, parsed) = FilesystemId::parse(&encoded).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(filesystem_id, parsed);
    }
}
