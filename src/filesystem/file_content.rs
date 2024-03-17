use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::{le_u64, le_u8};

use crate::codec::crypto::SymLockedAccessKey;
use crate::codec::ParserResult;
use crate::filesystem::ContentReference;

const FILE_CONTENT_TYPE_STUB: u8 = 0x01;

const FILE_CONTENT_TYPE_PUBLIC: u8 = 0x02;

const FILE_CONTENT_TYPE_ENCRYPTED: u8 = 0x03;

#[derive(Clone, Debug)]
pub enum FileContent {
    Encrypted {
        locked_access_key: SymLockedAccessKey,
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
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        match self {
            Self::Stub { size } => {
                writer.write_all(&[FILE_CONTENT_TYPE_STUB]).await?;
                written_bytes += 1;

                let size_bytes = size.to_le_bytes();
                writer.write_all(&size_bytes).await?;
                written_bytes += size_bytes.len();
            }
            Self::Public { content } => {
                writer.write_all(&[FILE_CONTENT_TYPE_PUBLIC]).await?;
                written_bytes += 1;

                written_bytes += encode_content_list(writer, content).await?;
            }
            Self::Encrypted {
                locked_access_key,
                content,
            } => {
                writer.write_all(&[FILE_CONTENT_TYPE_ENCRYPTED]).await?;
                written_bytes += 1;
                written_bytes += locked_access_key.encode(writer).await?;

                written_bytes += encode_content_list(writer, content).await?;
            }
        }

        Ok(written_bytes)
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted { .. })
    }

    pub fn parse<'a>(input: &'a [u8]) -> ParserResult<'a, Self> {
        let (input, content_type) = le_u8(input)?;

        let parsed = match content_type {
            FILE_CONTENT_TYPE_STUB => {
                let (input, size) = le_u64(input)?;
                (input, FileContent::Stub { size })
            }
            FILE_CONTENT_TYPE_PUBLIC => {
                let (input, ref_count) = le_u8(input)?;
                let (input, content) = ContentReference::parse_many(input, ref_count)?;

                (input, FileContent::Public { content })
            }
            FILE_CONTENT_TYPE_ENCRYPTED => {
                let (input, locked_access_key) = SymLockedAccessKey::parse(input)?;

                let (input, ref_count) = le_u8(input)?;
                let (input, content) = ContentReference::parse_many(input, ref_count)?;

                let data = FileContent::Encrypted {
                    locked_access_key,
                    content,
                };

                (input, data)
            }
            _ => {
                let err = nom::error::make_error(input, nom::error::ErrorKind::Tag);
                return Err(nom::Err::Failure(err));
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
