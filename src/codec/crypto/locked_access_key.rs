use chacha20poly1305::{AeadInPlace, Key as ChaChaKey, KeyInit, XChaCha20Poly1305};
use nom::bytes::streaming::take;
use nom::multi::count;
use nom::sequence::tuple;
use nom::{IResult, Needed};

use crate::codec::crypto::{
    AccessKey, AuthenticationTag, KeyId, Nonce, SigningKey, VerifyingKey, SYMMETRIC_KEY_LENGTH,
};
const ACCESS_KEY_CIPHER_TEXT_LENGTH: usize = SYMMETRIC_KEY_LENGTH + KEY_VERIFICATION_PATTERN_LENGTH;

const ACCESS_KEY_RECORD_LENGTH: usize = KeyId::size()
    + VerifyingKey::size()
    + Nonce::size()
    + ACCESS_KEY_CIPHER_TEXT_LENGTH
    + AuthenticationTag::size();

const KEY_VERIFICATION_PATTERN_LENGTH: usize = 4;

pub struct LockedAccessKey {
    pub(crate) key_id: KeyId,
    pub(crate) dh_exchange_key: VerifyingKey,
    pub(crate) nonce: Nonce,
    pub(crate) cipher_text: [u8; ACCESS_KEY_CIPHER_TEXT_LENGTH],
    pub(crate) tag: AuthenticationTag,
}

impl LockedAccessKey {
    pub(crate) fn unlock(
        &self,
        key: &SigningKey,
    ) -> Result<AccessKey, LockedAccessKeyError<&[u8]>> {
        if self.key_id != key.verifying_key().key_id() {
            return Err(LockedAccessKeyError::IncorrectKey);
        }

        let shared_secret = key.dh_exchange(&self.dh_exchange_key);
        let mut key_payload = self.cipher_text.to_vec();

        XChaCha20Poly1305::new(ChaChaKey::from_slice(&shared_secret))
            .decrypt_in_place_detached(&self.nonce, &[], &mut key_payload, &self.tag)
            .map_err(|_| LockedAccessKeyError::CryptoFailure)?;

        let mut key = [0u8; SYMMETRIC_KEY_LENGTH];
        key.copy_from_slice(&key_payload[..SYMMETRIC_KEY_LENGTH]);

        let mut verification_pattern = [0u8; KEY_VERIFICATION_PATTERN_LENGTH];
        verification_pattern.copy_from_slice(&key_payload[SYMMETRIC_KEY_LENGTH..]);

        if u32::from_le_bytes(verification_pattern) != 0 {
            return Err(LockedAccessKeyError::IncorrectKey);
        }

        Ok(AccessKey::from(key))
    }

    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (key_id, dh_exchange_key, nonce, cipher_text, tag)) = tuple((
            KeyId::parse,
            VerifyingKey::parse,
            Nonce::parse,
            take(ACCESS_KEY_CIPHER_TEXT_LENGTH),
            AuthenticationTag::parse,
        ))(input)?;

        let access_key = Self {
            key_id,
            dh_exchange_key,
            nonce,
            cipher_text: cipher_text.try_into().unwrap(),
            tag,
        };

        Ok((input, access_key))
    }

    pub(crate) fn parse_many(input: &[u8], key_count: u8) -> IResult<&[u8], Vec<Self>> {
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
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LockedAccessKeyError<I> {
    #[error("decoding data failed: {0}")]
    FormatFailure(#[from] nom::Err<nom::error::Error<I>>),

    #[error("unspecified crypto error")]
    CryptoFailure,

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,
}

impl<I> From<chacha20poly1305::Error> for LockedAccessKeyError<I> {
    fn from(_: chacha20poly1305::Error) -> Self {
        LockedAccessKeyError::CryptoFailure
    }
}
