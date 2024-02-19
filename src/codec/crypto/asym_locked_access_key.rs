use async_trait::async_trait;
use chacha20poly1305::{AeadInPlace, KeyInit, XChaCha20Poly1305};
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::multi::count;
use nom::sequence::tuple;
use nom::{IResult, Needed};

use crate::codec::crypto::{AccessKey, AuthenticationTag, KeyId, Nonce, SigningKey, VerifyingKey};
use crate::codec::AsyncEncodable;

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
    pub fn key_id(&self) -> KeyId {
        self.key_id
    }

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

    pub fn unlock(&self, key: &SigningKey) -> Result<AccessKey, AsymLockedAccessKeyError> {
        if self.key_id != key.verifying_key().key_id() {
            return Err(AsymLockedAccessKeyError::IncorrectKey);
        }

        let shared_secret = key.dh_exchange(&self.dh_exchange_key);
        let mut key_payload = self.cipher_text;

        XChaCha20Poly1305::new(shared_secret.chacha_key()).decrypt_in_place_detached(
            &self.nonce,
            &[],
            &mut key_payload,
            &self.tag,
        )?;

        Ok(AccessKey::from(key_payload))
    }
}

impl std::fmt::Debug for AsymLockedAccessKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AsymLockedAccessKey({:?})", self.key_id())
    }
}

#[async_trait]
impl AsyncEncodable for AsymLockedAccessKey {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += self.key_id.encode(writer).await?;
        written_bytes += self.dh_exchange_key.encode(writer).await?;
        written_bytes += self.nonce.encode(writer).await?;

        writer.write_all(&self.cipher_text).await?;
        written_bytes += self.cipher_text.len();

        written_bytes += self.tag.encode(writer).await?;

        Ok(written_bytes)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AsymLockedAccessKeyError {
    #[error("crypto error: {0}")]
    CryptoFailure(String),

    #[error("decoding data failed: {0}")]
    FormatFailure(String),

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,
}

impl From<chacha20poly1305::Error> for AsymLockedAccessKeyError {
    fn from(err: chacha20poly1305::Error) -> Self {
        AsymLockedAccessKeyError::CryptoFailure(err.to_string())
    }
}
