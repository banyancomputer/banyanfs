use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;

use crate::codec::ParserResult;

const ID_LENGTH: usize = 16;

#[derive(Clone, Copy, PartialEq)]
pub struct FilesystemId([u8; ID_LENGTH]);

impl FilesystemId {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        let current_ts = crate::utils::current_time_ms();
        let timestamp_bytes = current_ts.to_be_bytes();

        let mut random_bytes: [u8; 10] = [0u8; 10];
        rng.fill_bytes(&mut random_bytes);

        let mixed_version_rng = 0b0111_0000 | (random_bytes[0] & 0b0000_1111);
        let variant_rng = 0b1000_0000 | (random_bytes[1] & 0b0011_1111);

        let mut uuid_bytes: [u8; ID_LENGTH] = [0u8; ID_LENGTH];

        uuid_bytes[0..=5].copy_from_slice(&timestamp_bytes[2..]);
        uuid_bytes[6] = mixed_version_rng;
        uuid_bytes[7] = random_bytes[2];
        uuid_bytes[8] = variant_rng;
        uuid_bytes[9..].copy_from_slice(&random_bytes[3..]);

        Self(uuid_bytes)
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

        // todo(sstelfox): validate the UUID format...

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
