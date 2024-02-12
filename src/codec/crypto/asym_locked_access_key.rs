use chacha20poly1305::{AeadInPlace, Key as ChaChaKey, KeyInit, XChaCha20Poly1305};
use nom::bytes::streaming::take;
use nom::multi::count;
use nom::sequence::tuple;
use nom::{IResult, Needed};

use crate::codec::crypto::{AccessKey, AuthenticationTag, KeyId, Nonce, SigningKey, VerifyingKey};

const ACCESS_KEY_RECORD_LENGTH: usize = KeyId::size()
    + VerifyingKey::size()
    + Nonce::size()
    + AccessKey::size()
    + AuthenticationTag::size();

pub struct AsymLockedAccessKey {
    pub(crate) key_id: KeyId,
    pub(crate) dh_exchange_key: VerifyingKey,
    pub(crate) nonce: Nonce,
    pub(crate) cipher_text: [u8; AccessKey::size()],
    pub(crate) tag: AuthenticationTag,
}

impl AsymLockedAccessKey {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (key_id, dh_exchange_key, nonce, raw_cipher_text, tag)) = tuple((
            KeyId::parse,
            VerifyingKey::parse,
            Nonce::parse,
            // This is NOT being parsed into the target data type yet as its still encrypted. We'll
            // construct it when the contents are valid.
            take(AccessKey::size()),
            AuthenticationTag::parse,
        ))(input)?;

        let mut cipher_text = [0u8; AccessKey::size()];
        cipher_text.copy_from_slice(raw_cipher_text);

        let access_key = Self {
            key_id,
            dh_exchange_key,
            nonce,
            cipher_text,
            tag,
        };

        Ok((input, access_key))
    }

    pub fn parse_many(input: &[u8], key_count: u8) -> IResult<&[u8], Vec<Self>> {
        let (input, keys) = match count(Self::parse, key_count as usize)(input) {
            Ok(res) => res,
            Err(nom::Err::Incomplete(Needed::Size(_))) => {
                // If there wasn't enough data for one of the records, return how much more data we
                // _actually_ need before we can keep going.
                let total_size = key_count as usize * ACCESS_KEY_RECORD_LENGTH;
                return Err(nom::Err::Incomplete(Needed::new(total_size - input.len())));
            }
            Err(err) => return Err(err),
        };

        Ok((input, keys))
    }

    pub fn unlock(&self, key: &SigningKey) -> Result<AccessKey, AsymLockedAccessKeyError<&[u8]>> {
        if self.key_id != key.verifying_key().key_id() {
            return Err(AsymLockedAccessKeyError::IncorrectKey);
        }

        let shared_secret = key.dh_exchange(&self.dh_exchange_key);
        let mut key_payload = self.cipher_text.to_vec();

        XChaCha20Poly1305::new(ChaChaKey::from_slice(&shared_secret)).decrypt_in_place_detached(
            &self.nonce,
            &[],
            &mut key_payload,
            &self.tag,
        )?;

        let mut key = [0u8; AccessKey::size()];
        key.copy_from_slice(&key_payload);

        Ok(AccessKey::from(key))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AsymLockedAccessKeyError<I> {
    #[error("crypto error: {0}")]
    CryptoFailure(String),

    #[error("decoding data failed: {0}")]
    FormatFailure(#[from] nom::Err<nom::error::Error<I>>),

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,
}

impl<I> From<chacha20poly1305::Error> for AsymLockedAccessKeyError<I> {
    fn from(err: chacha20poly1305::Error) -> Self {
        AsymLockedAccessKeyError::CryptoFailure(err.to_string())
    }
}