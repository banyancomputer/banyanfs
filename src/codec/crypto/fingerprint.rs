use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;

use crate::codec::crypto::{KeyId, VerifyingKey};
use crate::codec::ParserResult;

const FINGERPRINT_SIZE: usize = 32;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Fingerprint([u8; FINGERPRINT_SIZE]);

impl Fingerprint {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }

    pub(crate) fn to_hex(&self) -> String {
        self.0
            .iter()
            .fold(String::new(), |acc, &b| format!("{acc}{:02x}", b))
    }

    pub fn key_id(&self) -> KeyId {
        let mut key_id = [0u8; 2];
        key_id.copy_from_slice(&self.0[..2]);
        KeyId::from(u16::from_le_bytes(key_id))
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (remaining, id_bytes) = take(FINGERPRINT_SIZE)(input)?;

        let mut bytes = [0u8; FINGERPRINT_SIZE];
        bytes.copy_from_slice(id_bytes);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        FINGERPRINT_SIZE
    }
}

impl std::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{0x{}}}", self.to_hex())
    }
}

impl From<[u8; FINGERPRINT_SIZE]> for Fingerprint {
    fn from(bytes: [u8; FINGERPRINT_SIZE]) -> Self {
        Self(bytes)
    }
}

impl From<&VerifyingKey> for Fingerprint {
    fn from(key: &VerifyingKey) -> Self {
        let public_key_bytes = key.to_encoded_point(true);
        Self(blake3::hash(public_key_bytes.as_bytes()).into())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    const REFERENCE_FINGERPRINT_BYTES: &[u8; 32] = b"UUUUUUUUaaaaaaaaUUUUUUUUaaaaaaaa";

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_fingerprint_debug_fmt() {
        let fingerprint = Fingerprint::from(*REFERENCE_FINGERPRINT_BYTES);
        let fmt_str = format!("{:?}", fingerprint);

        assert_eq!(
            fmt_str,
            "{0x5555555555555555616161616161616155555555555555556161616161616161}"
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_key_id_from_fingerprint() {
        let fingerprint = Fingerprint::from(*REFERENCE_FINGERPRINT_BYTES);
        let key_id = fingerprint.key_id();
        assert_eq!(key_id, KeyId::from(0x5555));
    }
}
