mod content_options;
mod history_end;
mod history_start;
mod key_access_settings;
mod permission_control;

pub use content_options::ContentOptions;
pub use history_end::HistoryEnd;
pub use history_start::HistoryStart;
pub use key_access_settings::KeyAccessSettings;
pub use permission_control::PermissionControl;

use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::error::{Error as NomError, ErrorKind};
use nom::number::streaming::le_u8;
use nom::{Err, IResult};

use crate::codec::crypto::{
    AccessKey, AsymLockedAccessKey, KeyId, Nonce, SigningKey, VerifyingKey,
};
use crate::codec::AsyncEncodable;
use crate::filesystem::PrivateEncodingContext;

use super::crypto::AuthenticationTag;

const ENCRYPTED_KEY_PAYLOAD_SIZE: usize = KeyId::size()
    + VerifyingKey::size()
    + Nonce::size()
    + AccessKey::size()
    + AuthenticationTag::size();

pub enum ContentPayload {
    Private,
    Public,
}

impl ContentPayload {
    pub async fn encode_private<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        context: &PrivateEncodingContext,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let mut keys = context
            .registered_keys
            .clone()
            .into_values()
            .collect::<Vec<_>>();

        keys.sort_by_key(|(pub_key, _)| pub_key.key_id());

        let keys_count = keys.len();
        if keys_count > 255 {
            return Err(IoError::new(
                IoErrorKind::InvalidInput,
                "too many keys in one filesystem to encode",
            ));
        }

        writer.write_all(&[keys_count as u8]).await?;
        written_bytes += 1;

        for (verifying_key, _) in keys.into_iter() {
            let escrowed_key = match context.key_access_key.lock_for(rng, &verifying_key) {
                Ok(vk) => vk,
                Err(err) => {
                    tracing::error!("failed to lock key for encoding: {}", err);
                    return Err(IoError::new(IoErrorKind::Other, "failed to lock key"));
                }
            };

            written_bytes += escrowed_key.encode(writer).await?;
        }

        // We need to build this part up and encrypt it before we write it out

        Ok(written_bytes)
    }

    pub fn parse_private<'a>(input: &'a [u8], key: &SigningKey) -> IResult<&'a [u8], Self> {
        let (input, key_count) = le_u8(input)?;
        let (input, locked_keys) = AsymLockedAccessKey::parse_many(input, key_count)?;

        let key_id = key.key_id();
        let relevant_keys = locked_keys.iter().filter(|k| k.key_id == key_id);

        let mut key_access_key = None;
        for potential_key in relevant_keys {
            if let Ok(key) = potential_key.unlock(key) {
                key_access_key = Some(key);
                break;
            }
        }

        let key_access_key = match key_access_key {
            Some(ak) => ak,
            None => return Err(Err::Failure(NomError::new(input, ErrorKind::Verify))),
        };

        let key_chunk_length = locked_keys.len() * ENCRYPTED_KEY_PAYLOAD_SIZE;
        let encrypted_chunk_length = HistoryStart::size() + key_chunk_length;

        let (input, nonce) = Nonce::parse(input)?;
        let (input, chunk) = take(encrypted_chunk_length)(input)?;
        let (input, tag) = AuthenticationTag::parse(input)?;

        let mut chunk = chunk.to_vec();
        key_access_key
            .decrypt_buffer(nonce, &mut chunk, tag)
            .map_err(|_| Err::Failure(NomError::new(input, ErrorKind::Verify)))?;

        // parse as a series of PermissionControl

        // todo(sstelfox): implement the rest

        Ok((input, ContentPayload::Private))
    }

    pub fn parse_public(_input: &[u8]) -> IResult<&[u8], Self> {
        todo!()
    }
}
