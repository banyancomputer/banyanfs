use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::multi::count;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;
use nom::{IResult, Needed};

use crate::crypto::utils::short_symmetric_decrypt;
use crate::crypto::{AccessKey, AuthenticationTag, CryptoError, Nonce};

const ESCROWED_ACCESS_RECORD_SIZE: usize = 148;

pub(crate) struct EscrowedAccessKey {
    nonce: Nonce,
    cipher_text: [u8; 36],
    tag: AuthenticationTag,
}

impl EscrowedAccessKey {
    pub(crate) fn assemble(nonce: Nonce, cipher_text: [u8; 36], tag: AuthenticationTag) -> Self {
        Self {
            nonce,
            cipher_text,
            tag,
        }
    }

    pub(crate) fn decrypt(
        &self,
        key: &AccessKey,
        aad: &[u8],
    ) -> Result<AccessKey, EncryptedPayloadError<&[u8]>> {
        let result = short_symmetric_decrypt(key, &self.nonce, &self.cipher_text, &self.tag, aad)
            .map_err(EncryptedPayloadError::CryptoFailure)?;

        let mut fixed_key: [u8; 32] = [0u8; 32];
        fixed_key.copy_from_slice(&result);

        Ok(AccessKey::from_bytes(fixed_key))
    }

    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        todo!()
    }

    pub(crate) fn parse_many(input: &[u8], key_count: u8) -> IResult<&[u8], Vec<Self>> {
        let (input, keys) = match count(Self::parse, key_count as usize)(input) {
            Ok(res) => res,
            Err(nom::Err::Incomplete(Needed::Size(_))) => {
                // If there wasn't enough data for one of the records, return how much more data we
                // _actually_ need before we can keep going.
                let total_size = key_count as usize * ESCROWED_ACCESS_RECORD_SIZE;

                return Err(nom::Err::Incomplete(Needed::new(total_size - input.len())));
            }
            Err(err) => return Err(err),
        };

        Ok((input, keys))
    }

    pub(crate) fn to_bytes(&self) -> [u8; 148] {
        let mut bytes = [0u8; 148];
        let mut current_idx = 0;

        let nonce_bytes = self.nonce.as_bytes();
        let nonce_len = nonce_bytes.len();
        bytes[current_idx..(current_idx + nonce_len)].copy_from_slice(nonce_bytes);
        current_idx += nonce_len;

        let cipher_len = self.cipher_text.len();
        bytes[current_idx..(current_idx + cipher_len)].copy_from_slice(&self.cipher_text);
        current_idx += cipher_len;

        let tag_bytes = self.tag.as_bytes();
        let tag_len = tag_bytes.len();
        bytes[current_idx..(current_idx + tag_len)].copy_from_slice(tag_bytes);

        bytes
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EncryptedPayloadError<I> {
    #[error("decoding data failed: {0}")]
    FormatFailure(#[from] nom::Err<nom::error::Error<I>>),

    #[error("crypto helper error: {0}")]
    CryptoFailure(#[from] CryptoError),

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,
}
