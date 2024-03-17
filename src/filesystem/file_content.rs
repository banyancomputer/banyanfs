use elliptic_curve::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::{le_u64, le_u8};

use crate::codec::crypto::{AccessKey, SymLockedAccessKey};
use crate::codec::{Cid, ParserResult};
use crate::filesystem::ContentReference;

const FILE_CONTENT_TYPE_STUB: u8 = 0x01;

const FILE_CONTENT_TYPE_PUBLIC: u8 = 0x02;

const FILE_CONTENT_TYPE_ENCRYPTED: u8 = 0x03;

#[derive(Clone, Debug)]
pub enum FileContent {
    Decrypted {
        access_key: AccessKey,
        content: Vec<ContentReference>,
    },
    Locked {
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
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        data_key: Option<&AccessKey>,
    ) -> std::io::Result<(usize, Vec<Cid>)> {
        let mut written_bytes = 0;

        let data_references = match self {
            Self::Stub { size } => {
                writer.write_all(&[FILE_CONTENT_TYPE_STUB]).await?;
                written_bytes += 1;

                let size_bytes = size.to_le_bytes();
                writer.write_all(&size_bytes).await?;
                written_bytes += size_bytes.len();

                Vec::new()
            }
            Self::Public { content } => {
                writer.write_all(&[FILE_CONTENT_TYPE_PUBLIC]).await?;
                written_bytes += 1;

                let (n, cid_list) = encode_content_list(writer, content).await?;
                written_bytes += n;

                cid_list
            }
            Self::Locked {
                locked_access_key,
                content,
            } => {
                writer.write_all(&[FILE_CONTENT_TYPE_ENCRYPTED]).await?;
                written_bytes += 1;
                written_bytes += locked_access_key.encode(writer).await?;

                let (n, cid_list) = encode_content_list(writer, content).await?;
                written_bytes += n;

                cid_list
            }
            Self::Decrypted {
                access_key,
                content,
            } => {
                writer.write_all(&[FILE_CONTENT_TYPE_ENCRYPTED]).await?;
                written_bytes += 1;

                let data_key = data_key.ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "data key required to encrypti private files",
                    )
                })?;

                let locked_key = access_key.lock_with(rng, data_key).map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "failed to lock access key",
                    )
                })?;

                written_bytes += locked_key.encode(writer).await?;

                let (n, cid_list) = encode_content_list(writer, content).await?;
                written_bytes += n;

                cid_list
            }
        };

        Ok((written_bytes, data_references))
    }

    pub fn is_encrypted(&self) -> bool {
        match &self {
            Self::Decrypted { .. } | Self::Locked { .. } => true,
            _ => false,
        }
    }

    pub fn parse<'a>(input: &'a [u8], access_key: Option<&AccessKey>) -> ParserResult<'a, Self> {
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

                let unlocked_key = match access_key {
                    Some(key) => {
                        let access_key = locked_access_key.unlock(key).map_err(|_| {
                            nom::Err::Failure(nom::error::make_error(
                                input,
                                nom::error::ErrorKind::Verify,
                            ))
                        })?;

                        Some(access_key)
                    }
                    None => None,
                };

                let (input, ref_count) = le_u8(input)?;
                let (input, content) = ContentReference::parse_many(input, ref_count)?;

                let data = match unlocked_key {
                    Some(access_key) => FileContent::Decrypted {
                        access_key,
                        content,
                    },
                    None => FileContent::Locked {
                        locked_access_key,
                        content,
                    },
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
            FileContent::Decrypted { content, .. } => content.iter().map(|c| c.size()).sum(),
            FileContent::Locked { content, .. } => content.iter().map(|c| c.size()).sum(),
            FileContent::Public { content } => content.iter().map(|c| c.size()).sum(),
            FileContent::Stub { size } => *size,
        }
    }
}

async fn encode_content_list<W: AsyncWrite + Unpin + Send>(
    writer: &mut W,
    content: &[ContentReference],
) -> std::io::Result<(usize, Vec<Cid>)> {
    let mut written_bytes = 0;

    let ref_count = content.len();
    if ref_count > u8::MAX as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "too many content references for a single file, redirect block required",
        ));
    }

    let mut cids = Vec::with_capacity(ref_count);
    for c in content {
        cids.push(c.data_block_cid());
        written_bytes += c.encode(writer).await?;
    }

    Ok((written_bytes, cids))
}
