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
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        mut pos: usize,
    ) -> std::io::Result<usize> {
        todo!()
    }
}
