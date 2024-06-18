use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{token::take, Parser};

use crate::codec::crypto::{AuthenticationTag, Nonce};
use crate::codec::{ParserResult, Stream};

const ENCRYPTED_BIT: u8 = 0b1000_0000;

/// Representation of Block parameters
///
/// The number of chunks in a block can exist in a range of 8-64
/// the size of a chunk can be 4kB-~134MB. Together this allows for blocks
/// to range in total size from 32kB to ~8.6GB
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataOptions {
    pub encrypted: bool,

    /// Representation of the number of chunks in a block.
    /// Valid in range 0-3
    /// Actual size is computed as 2^(3+chunk_count_exponent)
    /// The valid ranges map into a a valid chunk count of 8-64
    pub chunk_count_exponent: u8,

    /// Number of chunks dedicated to ECC, valid in range 0-15
    pub error_correction_count: u8,

    /// Representation of the size of an encrypted chunk, valid in the range 0-15.
    /// The actual size is computed as 2^(12+chunk_size_exponent).
    /// This maps to chunk sizes of 2^12 = 4kB to 2^27 = ~134MB
    pub chunk_size_exponent: u8,
}

impl DataOptions {
    pub const fn new(
        encrypted: bool,
        chunk_count_exponent: u8,
        error_correction_count: u8,
        chunk_size_exponent: u8,
    ) -> Result<Self, DataOptionsError> {
        if chunk_count_exponent > 3 {
            return Err(DataOptionsError::ChunkCountTooLarge(chunk_count_exponent));
        }
        if chunk_size_exponent > 15 {
            return Err(DataOptionsError::ChunkSizeTooLarge(chunk_size_exponent));
        }
        if error_correction_count > 15 {
            return Err(DataOptionsError::ErrorCorrectionCountTooLarge(
                error_correction_count,
            ));
        }
        Ok(Self {
            encrypted,
            chunk_count_exponent,
            error_correction_count,
            chunk_size_exponent,
        })
    }

    pub fn chunk_size(&self) -> u32 {
        2u32.pow(12 + u32::from(self.chunk_size_exponent))
    }

    pub fn chunk_count(&self) -> u8 {
        2u8.pow(1 + u32::from(self.chunk_count_exponent))
    }

    pub fn block_size(&self) -> u64 {
        u64::from(self.chunk_count()) * u64::from(self.chunk_size())
    }

    /// The amount of bytes a block can actually store across all of its chunks as data after accounting for the encryption overhead(if applicable)
    /// and the length field
    pub fn block_data_size(&self) -> usize {
        self.chunk_data_size() * self.chunk_count() as usize
    }

    /// Size of the contents of a chunk not including the encryption overhead (if applicable)
    pub fn chunk_payload_size(&self) -> usize {
        if self.encrypted {
            usize::try_from(self.chunk_size())
                .expect("Architectures below 32 bit are not supported")
                - (Nonce::size() + AuthenticationTag::size())
        } else {
            usize::try_from(self.chunk_size())
                .expect("Architectures below 32 bit are not supported")
        }
    }

    /// The amount of bytes a chunk can actually store as data after accounting for the encryption overhead(if applicable)
    /// and the length field
    pub fn chunk_data_size(&self) -> usize {
        // Subtracting 8 accounts for the 32 bit length field
        self.chunk_payload_size() - 4
    }

    pub fn ecc_present(&self) -> bool {
        self.error_correction_count > 0
    }

    ///  0           1          6             8           12           16
    /// || Encrypted  | Reserved | Chunk Count | ECC Count | Chunk Size ||
    ///       1 bit       5 bit       2 bit        4 bit       4 bit
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut encoded = [0u8; 2];

        if self.encrypted {
            encoded[0] |= ENCRYPTED_BIT;
        }
        encoded[0] |= self.chunk_count_exponent;

        encoded[1] |= self.error_correction_count << 4 | self.chunk_size_exponent;

        writer.write_all(&encoded).await?;
        Ok(encoded.len())
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, data_options) = take(2usize).parse_peek(input)?;

        let encrypted = data_options[0] & ENCRYPTED_BIT != 0;
        let chunk_count_exponent = data_options[0] & 0b11;
        let error_correction_count = (data_options[1] & 0b1111_0000) >> 4;
        let chunk_size_exponent = data_options[1] & 0b1111;

        let data_options = match DataOptions::new(
            encrypted,
            chunk_count_exponent,
            error_correction_count,
            chunk_size_exponent,
        ) {
            Ok(dopt) => dopt,
            Err(_) => {
                let err = winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Verify,
                );
                return Err(winnow::error::ErrMode::Cut(err));
            }
        };

        Ok((input, data_options))
    }

    pub const fn size() -> usize {
        2
    }

    pub fn small_encrypted_no_ecc() -> Self {
        DataOptions::new(true, 0, 0, 0).expect("We know this wont fail with these parameters")
    }

    pub fn standard_encrypted_no_ecc() -> Self {
        DataOptions::new(true, 3, 0, 8).expect("We know this wont fail with these parameters")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DataOptionsError {
    #[error("chunk size {0} is larger than max value 15")]
    ChunkSizeTooLarge(u8),

    #[error("attempted to add a chunk of size {0} to a block with max size of {1}")]
    ChunkTooLarge(usize, usize),

    #[error("error correction count {0} is larger than max value 15")]
    ErrorCorrectionCountTooLarge(u8),

    #[error("Chunk count {0} larger than max value 3")]
    ChunkCountTooLarge(u8),
}

#[cfg(test)]
mod test {
    use super::*;
    use winnow::Partial;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn encode_parse_round_trip_standard() {
        let original = DataOptions::standard_encrypted_no_ecc();
        let mut encoded = Vec::new();
        original.encode(&mut encoded).await.unwrap();
        assert_eq!(encoded.len(), DataOptions::size());
        let encoded = Partial::new(encoded.as_slice());
        let (remaining, parsed) = DataOptions::parse(encoded).unwrap();
        assert_eq!(parsed, original);
        assert!(remaining.is_empty())
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn encode_parse_round_trip_small() {
        let original = DataOptions::small_encrypted_no_ecc();
        let mut encoded = Vec::new();
        original.encode(&mut encoded).await.unwrap();
        assert_eq!(encoded.len(), DataOptions::size());
        let encoded = Partial::new(encoded.as_slice());
        let (remaining, parsed) = DataOptions::parse(encoded).unwrap();
        assert_eq!(parsed, original);
        assert!(remaining.is_empty())
    }
}
