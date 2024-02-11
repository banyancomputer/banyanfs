use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::error::{Error as NomError, ErrorKind};
use nom::number::streaming::{le_u64, le_u8};
use nom::IResult;
use time::OffsetDateTime;

use crate::codec::filesystem::Permissions;
use crate::codec::ActorId;
use crate::codec::AsyncEncodable;

const ATTRIBUTE_CUSTOM_TYPE_ID: u8 = 0x00;

const ATTRIBUTE_OWNER_TYPE_ID: u8 = 0x01;

const ATTRIBUTE_PERMISSIONS_TYPE_ID: u8 = 0x02;

const ATTRIBUTE_CREATED_AT_TYPE_ID: u8 = 0x03;

const ATTRIBUTE_MODIFIED_AT_TYPE_ID: u8 = 0x04;

const ATTRIBUTE_MIME_TYPE_TYPE_ID: u8 = 0x05;

pub enum Attribute {
    // Note: key and value both must encode to fewer than 255 bytes each
    Custom { key: String, value: String },

    Owner(ActorId),
    Permissions(Permissions),

    CreatedAt(OffsetDateTime),
    ModifiedAt(OffsetDateTime),

    MimeType(String),
}

impl Attribute {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (remaining, type_byte) = le_u8(input)?;

        let parsed = match type_byte {
            ATTRIBUTE_CUSTOM_TYPE_ID => {
                let (remaining, (key_len, value_len)) =
                    nom::sequence::pair(le_u8, le_u8)(remaining)?;

                let (remaining, key_bytes) = take(key_len)(remaining)?;
                let key = String::from_utf8(key_bytes.to_vec())
                    .map_err(|_| nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;

                let (remaining, value_bytes) = take(value_len)(remaining)?;
                let value = String::from_utf8(value_bytes.to_vec())
                    .map_err(|_| nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;

                (remaining, Self::Custom { key, value })
            }
            ATTRIBUTE_OWNER_TYPE_ID => {
                let (remaining, actor_id) = ActorId::parse(remaining)?;
                (remaining, Self::Owner(actor_id))
            }
            ATTRIBUTE_PERMISSIONS_TYPE_ID => {
                let (remaining, permissions) = Permissions::parse(remaining)?;
                (remaining, Self::Permissions(permissions))
            }
            ATTRIBUTE_CREATED_AT_TYPE_ID => {
                let (remaining, unix_milliseconds) = le_u64(remaining)?;

                let unix_nanos = unix_milliseconds as i128 * 1_000_000;
                let time = OffsetDateTime::from_unix_timestamp_nanos(unix_nanos)
                    .map_err(|_| nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;

                (remaining, Self::CreatedAt(time))
            }
            ATTRIBUTE_MODIFIED_AT_TYPE_ID => {
                let (remaining, unix_milliseconds) = le_u64(remaining)?;

                let unix_nanos = unix_milliseconds as i128 * 1_000_000;
                let time = OffsetDateTime::from_unix_timestamp_nanos(unix_nanos)
                    .map_err(|_| nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;

                (remaining, Self::ModifiedAt(time))
            }
            ATTRIBUTE_MIME_TYPE_TYPE_ID => {
                let (remaining, mime_len) = le_u8(remaining)?;

                let (remaining, mime_bytes) = take(mime_len)(remaining)?;
                let mime = String::from_utf8(mime_bytes.to_vec())
                    .map_err(|_| nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;

                (remaining, Self::MimeType(mime))
            }
            _ => return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Tag))),
        };

        Ok(parsed)
    }
}

#[async_trait]
impl AsyncEncodable for Attribute {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        pos: usize,
    ) -> std::io::Result<usize> {
        match self {
            Self::Custom { key, value } => {
                let key_bytes = key.as_bytes();
                let key_len = key_bytes.len();

                let value_bytes = value.as_bytes();
                let value_len = value_bytes.len();

                if key_len > 255 || value_len > 255 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "attribute key or value longer than 255 bytes when encoded",
                    ));
                }

                writer.write_all(&[ATTRIBUTE_CUSTOM_TYPE_ID]).await?;
                writer.write_all(&[key_len as u8, value_len as u8]).await?;
                writer.write_all(key_bytes).await?;
                writer.write_all(value_bytes).await?;

                Ok(pos + 1 + 2 + key_len + value_len)
            }
            Self::Owner(actor_id) => {
                writer.write_all(&[ATTRIBUTE_OWNER_TYPE_ID]).await?;
                actor_id.encode(writer, pos + 1).await
            }
            Self::Permissions(permissions) => {
                writer.write_all(&[ATTRIBUTE_PERMISSIONS_TYPE_ID]).await?;
                permissions.encode(writer, pos + 1).await
            }
            Self::CreatedAt(time) => {
                writer.write_all(&[ATTRIBUTE_CREATED_AT_TYPE_ID]).await?;

                let unix_milliseconds: u64 = (time.unix_timestamp_nanos() / 1_000_000) as u64;
                let ts_bytes = unix_milliseconds.to_le_bytes();
                writer.write_all(&ts_bytes).await?;

                Ok(pos + 1 + 8)
            }
            Self::ModifiedAt(time) => {
                writer.write_all(&[ATTRIBUTE_MODIFIED_AT_TYPE_ID]).await?;

                let unix_milliseconds: u64 = (time.unix_timestamp_nanos() / 1_000_000) as u64;
                let ts_bytes = unix_milliseconds.to_le_bytes();
                writer.write_all(&ts_bytes).await?;

                Ok(pos + 1 + 8)
            }
            Self::MimeType(mime) => {
                let mime_bytes = mime.as_bytes();
                let mime_len = mime_bytes.len();

                if mime_len > 255 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "mime type longer than 255 bytes when encoded",
                    ));
                }

                writer.write_all(&[ATTRIBUTE_MIME_TYPE_TYPE_ID]).await?;
                writer.write_all(mime_bytes).await?;

                Ok(pos + 1 + mime_len)
            }
        }
    }
}
