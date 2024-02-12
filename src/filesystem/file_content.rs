use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::{le_u64, le_u8};

use crate::codec::crypto::SymLockedAccessKey;
use crate::codec::AsyncEncodable;
use crate::filesystem::ContentReference;

const FILE_CONTENT_TYPE_STUB: u8 = 0x01;

const FILE_CONTENT_TYPE_PUBLIC: u8 = 0x02;

const FILE_CONTENT_TYPE_ENCRYPTED: u8 = 0x03;

#[derive(Clone)]
pub enum FileContent {
    Encrypted {
        access_key: SymLockedAccessKey,
        content: Vec<ContentReference>,
    },
    Public {
        content: Vec<ContentReference>,
    },
    Stub {
        size: u64,
    },
}

impl FileContent {
    pub fn is_encrypted(&self) -> bool {
        matches!(self, FileContent::Encrypted { .. })
    }

    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, content_type) = le_u8(input)?;

        let parsed = match content_type {
            FILE_CONTENT_TYPE_STUB => {
                let (remaining, size) = le_u64(remaining)?;
                (remaining, FileContent::Stub { size })
            }
            FILE_CONTENT_TYPE_PUBLIC => {
                let (remaining, ref_count) = le_u8(remaining)?;
                let (remaining, content) = ContentReference::parse_many(remaining, ref_count)?;
                (remaining, FileContent::Public { content })
            }
            FILE_CONTENT_TYPE_ENCRYPTED => {
                let (remaining, access_key) = SymLockedAccessKey::parse(remaining)?;
                let (remaining, ref_count) = le_u8(remaining)?;
                let (remaining, content) = ContentReference::parse_many(remaining, ref_count)?;
                (
                    remaining,
                    FileContent::Encrypted {
                        access_key,
                        content,
                    },
                )
            }
            _ => {
                return Err(nom::Err::Error(NomError::new(input, ErrorKind::Tag)));
            }
        };

        Ok(parsed)
    }

    pub fn size(&self) -> u64 {
        match self {
            FileContent::Encrypted { content, .. } => content.iter().map(|c| c.size()).sum(),
            FileContent::Public { content } => content.iter().map(|c| c.size()).sum(),
            FileContent::Stub { size } => *size,
        }
    }
}

#[async_trait]
impl AsyncEncodable for FileContent {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        match self {
            Self::Encrypted {
                access_key,
                content,
            } => {
                writer.write_all(&[FILE_CONTENT_TYPE_PUBLIC]).await?;
                written_bytes += 1;

                written_bytes += access_key.encode(writer).await?;
                written_bytes += encode_content_list(writer, content).await?;
            }
            Self::Public { content } => {
                writer.write_all(&[FILE_CONTENT_TYPE_PUBLIC]).await?;
                written_bytes += 1;

                written_bytes += encode_content_list(writer, content).await?;
            }
            Self::Stub { size } => {
                writer.write_all(&[FILE_CONTENT_TYPE_STUB]).await?;
                written_bytes += 1;

                let size_bytes = size.to_le_bytes();
                writer.write_all(&size_bytes).await?;
                written_bytes += size_bytes.len();
            }
        }

        Ok(written_bytes)
    }
}

async fn encode_content_list<W: AsyncWrite + Unpin + Send>(
    writer: &mut W,
    content: &[ContentReference],
) -> std::io::Result<usize> {
    let mut written_bytes = 0;

    let ref_count = content.len();
    if ref_count > u8::MAX as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "too many content references for a single file, redirect block required",
        ));
    }

    for c in content {
        written_bytes += c.encode(writer).await?;
    }

    Ok(written_bytes)
}
