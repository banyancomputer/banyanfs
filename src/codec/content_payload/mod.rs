mod content_options;
mod history_end;
mod history_start;
mod key_access_settings;

pub use content_options::ContentOptions;
pub use history_end::HistoryEnd;
pub use history_start::HistoryStart;
pub use key_access_settings::KeyAccessSettings;

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::AsyncWrite;
use nom::error::{Error as NomError, ErrorKind};
use nom::number::streaming::le_u8;
use nom::{Err, IResult};

use crate::codec::crypto::{AsymLockedAccessKey, SigningKey};

pub enum ContentPayload {
    Private,
    Public,
}

impl ContentPayload {
    pub async fn encode_private<W: AsyncWrite + Unpin + Send>(
        &self,
        _rng: &mut impl CryptoRngCore,
        _writer: &mut W,
    ) -> std::io::Result<usize> {
        let _written_bytes = 0;

        //todo: it may make sense to still allow private data encryption in public filesystems...

        todo!();

        //Ok(written_bytes)
    }

    pub fn parse_private<'a>(input: &'a [u8], key: &SigningKey) -> IResult<&'a [u8], Self> {
        let (input, key_count) = le_u8(input)?;
        let (input, locked_keys) = AsymLockedAccessKey::parse_many(input, key_count)?;

        let key_id = key.key_id();
        let relevant_keys = locked_keys.into_iter().filter(|k| k.key_id == key_id);

        let mut key_access_key = None;
        for potential_key in relevant_keys {
            if let Ok(key) = potential_key.unlock(key) {
                key_access_key = Some(key);
                break;
            }
        }

        let _key_access_key = match key_access_key {
            Some(ak) => ak,
            None => return Err(Err::Failure(NomError::new(input, ErrorKind::Verify))),
        };

        // todo(sstelfox): implement the rest

        Ok((input, ContentPayload::Private))
    }

    pub fn parse_public(_input: &[u8]) -> IResult<&[u8], Self> {
        todo!()
    }
}
