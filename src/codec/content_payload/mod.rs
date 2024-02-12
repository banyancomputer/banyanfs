mod content_options;
mod history_end;
mod history_start;
mod key_access_settings;

pub use content_options::ContentOptions;
pub use history_end::HistoryEnd;
pub use history_start::HistoryStart;
pub use key_access_settings::KeyAccessSettings;

use async_trait::async_trait;
use futures::AsyncWrite;
use nom::error::{Error as NomError, ErrorKind};
use nom::number::streaming::le_u8;
use nom::{Err, IResult};

use crate::codec::crypto::{AccessKey, AsymLockedAccessKey, SigningKey};
use crate::codec::AsyncEncodable;

pub enum ContentPayload {
    Private { access_key: AccessKey },
    Public,
}

impl ContentPayload {
    pub fn parse_private<'a>(input: &'a [u8], key: &SigningKey) -> IResult<&'a [u8], Self> {
        let (input, key_count) = le_u8(input)?;
        let (input, locked_keys) = AsymLockedAccessKey::parse_many(input, key_count)?;

        let key_id = key.key_id();
        let relevant_keys = locked_keys.into_iter().filter(|k| k.key_id == key_id);

        let mut access_key = None;
        for potential_key in relevant_keys {
            if let Ok(key) = potential_key.unlock(key) {
                access_key = Some(key);
                break;
            }
        }

        let access_key = match access_key {
            Some(ak) => ak,
            None => return Err(Err::Failure(NomError::new(input, ErrorKind::Verify))),
        };

        // todo(sstelfox): implement the rest

        Ok((input, ContentPayload::Private { access_key }))
    }

    pub fn parse_public(input: &[u8]) -> IResult<&[u8], Self> {
        // todo(sstelfox): implement
        Ok((input, ContentPayload::Public))
    }
}

#[async_trait]
impl AsyncEncodable for ContentPayload {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        _writer: &mut W,
        _pos: usize,
    ) -> std::io::Result<usize> {
        let _written_bytes = 0;

        todo!();

        //Ok(written_bytes)
    }
}
