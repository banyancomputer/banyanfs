use nom::error::{Error as NomError, ErrorKind};
use nom::number::streaming::le_u8;
use nom::{Err, IResult};

use crate::codec::crypto::{AccessKey, LockedAccessKey, SigningKey};

#[allow(dead_code)]
pub(crate) enum ContentPayload {
    Private { access_key: AccessKey },
    Public,
}

impl ContentPayload {
    #[allow(dead_code)]
    pub fn parse_private<'a>(input: &'a [u8], key: &SigningKey) -> IResult<&'a [u8], Self> {
        let (input, key_count) = le_u8(input)?;
        let (input, locked_keys) = LockedAccessKey::parse_many(input, key_count)?;

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

        Ok((input, ContentPayload::Private { access_key }))
    }

    #[allow(dead_code)]
    pub fn parse_public(input: &[u8]) -> IResult<&[u8], Self> {
        Ok((input, ContentPayload::Public))
    }
}
