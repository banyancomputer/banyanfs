use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::error::Error as NomError;
use nom::error::ErrorKind;

use crate::codec::AsyncEncodable;

const ECC_PRESENT_BIT: u8 = 0x02;

const PRIVATE_BIT: u8 = 0x01;

const RESERVED_BITS: u8 = 0xfc;

#[derive(Debug, PartialEq)]
pub struct PublicSettings {
    ecc_present: bool,
    private: bool,
}

impl PublicSettings {
    pub fn ecc_present(&self) -> bool {
        self.ecc_present
    }

    pub fn new(ecc_present: bool, private: bool) -> Self {
        Self {
            ecc_present,
            private,
        }
    }

    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, settings_byte) = take(1u8)(input)?;
        let settings_byte = settings_byte[0];

        if cfg!(feature = "strict") && (settings_byte & RESERVED_BITS) != 0 {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)));
        }

        let ecc_present = (settings_byte & ECC_PRESENT_BIT) == ECC_PRESENT_BIT;
        let private = (settings_byte & PRIVATE_BIT) == PRIVATE_BIT;

        let settings = Self {
            ecc_present,
            private,
        };

        Ok((input, settings))
    }

    pub fn private(&self) -> bool {
        self.private
    }
}

#[async_trait]
impl AsyncEncodable for PublicSettings {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        pos: usize,
    ) -> std::io::Result<usize> {
        let mut settings_byte = 0;

        if self.ecc_present {
            settings_byte |= ECC_PRESENT_BIT;
        }

        if self.private {
            settings_byte |= PRIVATE_BIT;
        }

        writer.write_all(&[settings_byte]).await?;
        Ok(pos + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip_public_noecc() {
        // Manually construct a correct header according to the RFC
        let source = vec![0b0000_0000];

        let (remaining, parsed) = PublicSettings::parse(&source).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(
            parsed,
            PublicSettings {
                ecc_present: false,
                private: false
            }
        );

        let mut encoded = Vec::new();
        parsed.encode(&mut encoded, 0).await.unwrap();
        assert_eq!(source, encoded);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip_public_ecc() {
        // Manually construct a correct header according to the RFC
        let source = vec![0b0000_0010];

        let (remaining, parsed) = PublicSettings::parse(&source).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(
            parsed,
            PublicSettings {
                ecc_present: true,
                private: false
            }
        );

        let mut encoded = Vec::new();
        parsed.encode(&mut encoded, 0).await.unwrap();
        assert_eq!(source, encoded);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip_private_noecc() {
        // Manually construct a correct header according to the RFC
        let source = vec![0b0000_0001];

        let (remaining, parsed) = PublicSettings::parse(&source).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(
            parsed,
            PublicSettings {
                ecc_present: false,
                private: true,
            }
        );

        let mut encoded = Vec::new();
        parsed.encode(&mut encoded, 0).await.unwrap();
        assert_eq!(source, encoded);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_round_trip_private_ecc() {
        // Manually construct a correct header according to the RFC
        let source = vec![0b0000_0011];

        let (remaining, parsed) = PublicSettings::parse(&source).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(
            parsed,
            PublicSettings {
                ecc_present: true,
                private: true,
            }
        );

        let mut encoded = Vec::new();
        parsed.encode(&mut encoded, 0).await.unwrap();
        assert_eq!(source, encoded);
    }

    #[cfg(feature = "strict")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_invalid() {
        let source = vec![0b0100_0000];
        assert!(PublicSettings::parse(&source).is_err());
    }
}
