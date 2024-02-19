use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::number::streaming::le_u8;

use crate::codec::crypto::{AccessKey, AsymLockedAccessKey, SigningKey};
use crate::codec::AsyncEncodable;

const KEY_PRESENT_BIT: u8 = 0b0000_0001;

pub struct PermissionKeys {
    filesystem: Option<AccessKey>,
    data: Option<AccessKey>,
    maintenance: Option<AccessKey>,
}

impl PermissionKeys {
    pub fn parse<'a>(input: &'a [u8], unlock_key: &SigningKey) -> nom::IResult<&'a [u8], Self> {
        let (input, filesystem) = maybe_parse_key(input)?;
        let filesystem = filesystem
            .map(|key| key.unlock(unlock_key))
            .transpose()
            .map_err(|_| {
                nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Verify))
            })?;

        let (input, data) = maybe_parse_key(input)?;
        let data = data
            .map(|key| key.unlock(unlock_key))
            .transpose()
            .map_err(|_| {
                nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Verify))
            })?;

        let (input, maintenance) = maybe_parse_key(input)?;
        let maintenance = maintenance
            .map(|key| key.unlock(unlock_key))
            .transpose()
            .map_err(|_| {
                nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Verify))
            })?;

        let permission_keys = Self {
            filesystem,
            data,
            maintenance,
        };

        Ok((input, permission_keys))
    }
}

#[async_trait]
impl AsyncEncodable for PermissionKeys {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        todo!("PermissionKeys::encode")
    }
}

fn maybe_parse_key(input: &[u8]) -> nom::IResult<&[u8], Option<AsymLockedAccessKey>> {
    let (input, presence_flag) = le_u8(input)?;

    if presence_flag & KEY_PRESENT_BIT != 0 {
        let (input, key) = AsymLockedAccessKey::parse(input)?;
        Ok((input, Some(key)))
    } else {
        // still need to advance the input
        let (input, _blank) = take(AccessKey::size())(input)?;
        Ok((input, None))
    }
}
