use futures::{AsyncWrite, AsyncWriteExt};
use winnow::binary::{le_u64, le_u8};
use winnow::Parser;

use crate::codec::crypto::SymLockedAccessKey;
use crate::codec::{Cid, ParserResult, Stream};
use crate::filesystem::ContentReference;

const FILE_CONTENT_TYPE_STUB: u8 = 0x01;

const FILE_CONTENT_TYPE_PUBLIC: u8 = 0x02;

const FILE_CONTENT_TYPE_ENCRYPTED: u8 = 0x03;

// todo(sstelfox): need to rename NodeContent...
#[derive(Clone, Debug)]
pub enum FileContent {
    Encrypted {
        locked_access_key: SymLockedAccessKey,
        cid: Cid,
        data_size: u64,
        content: Vec<ContentReference>,
    },
    Public {
        cid: Cid,
        data_size: u64,
        content: Vec<ContentReference>,
    },
    Stub {
        data_size: u64,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum FileContentError {
    #[error("node type does not contain any content")]
    NoContent,

    #[error("key requested on unencrypted data")]
    NotEncrypted,
}

impl FileContent {
    pub fn encrypted(
        access_key: SymLockedAccessKey,
        cid: Cid,
        data_size: u64,
        content: Vec<ContentReference>,
    ) -> Self {
        Self::Encrypted {
            locked_access_key: access_key,
            cid,
            data_size,
            content,
        }
    }

    /// Return the CID over the plaintext content of the file. Be careful not to confuse these with
    /// the data CIDs which are the CIDs of the stored data blocks.
    pub fn cid(&self) -> Option<Cid> {
        match self {
            Self::Encrypted { cid, .. } | Self::Public { cid, .. } => Some(cid.clone()),
            Self::Stub { .. } => None,
        }
    }

    pub fn content_references(&self) -> Result<&[ContentReference], FileContentError> {
        match self {
            Self::Encrypted { content, .. } | Self::Public { content, .. } => Ok(content),
            Self::Stub { .. } => Err(FileContentError::NoContent),
        }
    }

    pub fn data_cids(&self) -> Option<Vec<Cid>> {
        match self {
            Self::Encrypted { content, .. } | Self::Public { content, .. } => {
                Some(content.iter().map(|c| c.data_block_cid()).collect())
            }
            Self::Stub { .. } => None,
        }
    }

    pub fn data_key(&self) -> Result<&SymLockedAccessKey, FileContentError> {
        match self {
            Self::Encrypted {
                locked_access_key, ..
            } => Ok(locked_access_key),
            _ => Err(FileContentError::NotEncrypted),
        }
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        match self {
            Self::Stub { data_size } => {
                writer.write_all(&[FILE_CONTENT_TYPE_STUB]).await?;
                written_bytes += 1;

                let data_size_bytes = data_size.to_le_bytes();
                writer.write_all(&data_size_bytes).await?;
                written_bytes += data_size_bytes.len();
            }
            Self::Public {
                cid,
                data_size,
                content,
            } => {
                writer.write_all(&[FILE_CONTENT_TYPE_PUBLIC]).await?;
                written_bytes += 1;
                written_bytes += cid.encode(writer).await?;

                let data_size_bytes = data_size.to_le_bytes();
                writer.write_all(&data_size_bytes).await?;
                written_bytes += data_size_bytes.len();

                written_bytes += encode_content_list(writer, content).await?;
            }
            Self::Encrypted {
                locked_access_key,
                cid,
                data_size,
                content,
            } => {
                writer.write_all(&[FILE_CONTENT_TYPE_ENCRYPTED]).await?;
                written_bytes += 1;
                written_bytes += cid.encode(writer).await?;

                let data_size_bytes = data_size.to_le_bytes();
                writer.write_all(&data_size_bytes).await?;
                written_bytes += data_size_bytes.len();

                written_bytes += locked_access_key.encode(writer).await?;
                written_bytes += encode_content_list(writer, content).await?;
            }
        }

        Ok(written_bytes)
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted { .. })
    }

    pub fn is_stub(&self) -> bool {
        matches!(self, Self::Stub { .. })
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, content_type) = le_u8.parse_peek(input)?;

        let parsed = match content_type {
            FILE_CONTENT_TYPE_STUB => {
                let (input, data_size) = le_u64.parse_peek(input)?;
                (input, FileContent::Stub { data_size })
            }
            FILE_CONTENT_TYPE_PUBLIC => {
                let (input, cid) = Cid::parse(input)?;
                let (input, data_size) = le_u64.parse_peek(input)?;
                let (input, ref_count) = le_u8.parse_peek(input)?;
                let (input, content) = ContentReference::parse_many(input, ref_count)?;

                (
                    input,
                    FileContent::Public {
                        cid,
                        data_size,
                        content,
                    },
                )
            }
            FILE_CONTENT_TYPE_ENCRYPTED => {
                let (input, cid) = Cid::parse(input)?;
                let (input, data_size) = le_u64.parse_peek(input)?;
                let (input, locked_access_key) = SymLockedAccessKey::parse(input)?;

                let (input, ref_count) = le_u8.parse_peek(input)?;
                let (input, content) = ContentReference::parse_many(input, ref_count)?;

                let data = FileContent::Encrypted {
                    cid,
                    data_size,
                    locked_access_key,
                    content,
                };

                (input, data)
            }
            _ => {
                let err = winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Tag,
                );
                return Err(winnow::error::ErrMode::Cut(err));
            }
        };

        Ok(parsed)
    }

    pub fn size(&self) -> u64 {
        match self {
            FileContent::Encrypted { data_size, .. } => *data_size,
            FileContent::Public { data_size, .. } => *data_size,
            FileContent::Stub { data_size } => *data_size,
        }
    }
}

async fn encode_content_list<W: AsyncWrite + Unpin + Send>(
    writer: &mut W,
    content: &[ContentReference],
) -> std::io::Result<usize> {
    let ref_count = content.len();
    if ref_count > u8::MAX as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "too many content references for a single file, redirect block required",
        ));
    }

    writer.write_all(&[ref_count as u8]).await?;
    let mut written_bytes = 1;

    for c in content {
        written_bytes += c.encode(writer).await?;
    }

    Ok(written_bytes)
}
