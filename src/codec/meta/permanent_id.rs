use ecdsa::signature::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use rand::Rng;
use winnow::{token::take, Parser};

use crate::codec::{ParserResult, Stream};

const PERMANENT_ID_SIZE: usize = 8;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PermanentId([u8; PERMANENT_ID_SIZE]);

impl PermanentId {
    pub fn as_bytes(&self) -> &[u8; PERMANENT_ID_SIZE] {
        &self.0
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(self.0.len())
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        Self(rng.gen())
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (remaining, id_bytes) = take(PERMANENT_ID_SIZE).parse_peek(input)?;

        let mut bytes = [0u8; PERMANENT_ID_SIZE];
        bytes.copy_from_slice(id_bytes);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        PERMANENT_ID_SIZE
    }
}

impl std::fmt::Debug for PermanentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id_str: String = self
            .0
            .iter()
            .fold(String::new(), |acc, &b| format!("{acc}{:02x}", b));

        write!(f, "PermanentId(0x{id_str})")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::rngs::OsRng;

    impl PermanentId {
        pub fn from_bytes(bytes: [u8; 8]) -> Self {
            Self(bytes)
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_permanent_id_round_trip_random() {
        let mut rng = OsRng;
        let original_id = PermanentId::generate(&mut rng);

        let mut buffer = Vec::new();
        original_id.encode(&mut buffer).await.unwrap();

        let (remaining, parsed_id) = PermanentId::parse(Stream::new(&buffer)).unwrap();

        assert_eq!(Vec::<u8>::new(), remaining.to_vec());
        assert_eq!(original_id, parsed_id);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_permanent_id_round_trip_hardcoded() {
        for val in [0u8, 1u8, 255u8] {
            let original_id = PermanentId::from_bytes([val; PERMANENT_ID_SIZE]);

            let mut buffer = Vec::new();
            original_id.encode(&mut buffer).await.unwrap();

            let (remaining, parsed_id) = PermanentId::parse(Stream::new(&buffer)).unwrap();

            assert_eq!(Vec::<u8>::new(), remaining.to_vec());

            assert_eq!(original_id, parsed_id);
            assert_eq!(original_id.as_bytes(), &[val; PERMANENT_ID_SIZE]);
        }
    }
}
