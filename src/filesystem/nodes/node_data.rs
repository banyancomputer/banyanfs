use std::collections::HashMap;

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::{le_u16, le_u64, le_u8};

use crate::codec::crypto::AccessKey;
use crate::codec::filesystem::{DirectoryPermissions, FilePermissions};
use crate::codec::{Cid, ParserResult, PermanentId};
use crate::filesystem::nodes::{NodeKind, NodeName};
use crate::filesystem::FileContent;

pub enum NodeData {
    File {
        permissions: FilePermissions,
        content: FileContent,
        associated_data: HashMap<u16, PermanentId>,
    },
    AssoicatedData,
    Directory {
        permissions: DirectoryPermissions,
        children: HashMap<NodeName, PermanentId>,
        children_size: u64,
    },
}

impl NodeData {
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        data_key: Option<&AccessKey>,
    ) -> std::io::Result<(usize, Option<Vec<PermanentId>>, Option<Vec<Cid>>)> {
        let mut written_bytes = 0;

        written_bytes += self.kind().encode(writer).await?;

        match &self {
            NodeData::File {
                permissions,
                content,
                associated_data,
            } => {
                written_bytes += permissions.encode(writer).await?;
                let (n, data_cids) = content.encode(rng, writer, data_key).await?;
                written_bytes += n;

                let ad_length = associated_data.len();
                if ad_length > u8::MAX as usize {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "too many associated data items with a single file entry",
                    ));
                }
                writer.write_all(&[ad_length as u8]).await?;
                written_bytes += 1;

                if associated_data.is_empty() {
                    return Ok((written_bytes, None, data_cids));
                }

                let mut child_ids = Vec::new();

                let mut ad_list = associated_data.iter().collect::<Vec<_>>();
                ad_list.sort();
                let mut highest_seen_ad_kind = 0;

                for (ad_kind, ad_perm_id) in ad_list.into_iter() {
                    if *ad_kind <= highest_seen_ad_kind {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "associated data kinds may only appear once a piece",
                        ));
                    }

                    highest_seen_ad_kind = *ad_kind;
                    writer.write_all(&ad_kind.to_le_bytes()).await?;
                    written_bytes += 2;
                    written_bytes += ad_perm_id.encode(writer).await?;

                    child_ids.push(*ad_perm_id);
                }

                Ok((written_bytes, Some(child_ids), data_cids))
            }
            NodeData::Directory {
                permissions,
                children_size,
                children,
            } => {
                written_bytes += permissions.encode(writer).await?;

                let children_size_bytes = children_size.to_le_bytes();
                writer.write_all(&children_size_bytes).await?;
                written_bytes += children_size_bytes.len();

                let child_count = children.len();
                if child_count > u16::MAX as usize {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "too many children in a single directory entry",
                    ));
                }

                let child_count_bytes = (child_count as u16).to_le_bytes();
                writer.write_all(&child_count_bytes).await?;
                written_bytes += child_count_bytes.len();

                let mut children = children.iter().collect::<Vec<_>>();
                children.sort_by(|(_, a), (_, b)| a.cmp(b));

                let mut children_ids = Vec::new();
                for (name, id) in children {
                    written_bytes += name.encode(writer).await?;
                    children_ids.push(*id);
                }

                Ok((written_bytes, Some(children_ids), None))
            }
            _ => unimplemented!(),
        }
    }

    pub(crate) fn kind(&self) -> NodeKind {
        match self {
            NodeData::File { .. } => NodeKind::File,
            NodeData::AssoicatedData => NodeKind::AssociatedData,
            NodeData::Directory { .. } => NodeKind::Directory,
        }
    }

    pub fn new_directory() -> Self {
        Self::Directory {
            permissions: DirectoryPermissions::default(),
            children: HashMap::new(),
            children_size: 0,
        }
    }

    pub(crate) fn parse<'a>(
        input: &'a [u8],
        data_key: Option<&AccessKey>,
    ) -> ParserResult<'a, Self> {
        let (input, kind) = NodeKind::parse(input)?;

        let (input, node_data) = match kind {
            NodeKind::File => {
                let (data_buf, permissions) = FilePermissions::parse(input)?;
                let (data_buf, content) = FileContent::parse(data_buf, data_key)?;

                let (data_buf, associated_data) = {
                    let (mut data_buf, ad_length) = le_u8(data_buf)?;
                    let mut associated_data = HashMap::new();

                    for _ in 0..ad_length {
                        let (ad_buf, ad_kind) = le_u16(data_buf)?;
                        let (ad_buf, ad_perm_id) = PermanentId::parse(ad_buf)?;
                        associated_data.insert(ad_kind, ad_perm_id);
                        data_buf = ad_buf;
                    }

                    (data_buf, associated_data)
                };

                let data = NodeData::File {
                    permissions,
                    content,
                    associated_data,
                };

                (data_buf, data)
            }
            //NodeKind::AssociatedData => {}
            NodeKind::Directory => {
                let (data_buf, permissions) = DirectoryPermissions::parse(input)?;
                let (mut data_buf, children_size) = le_u64(data_buf)?;

                let mut children = HashMap::new();
                for _ in 0..children_size {
                    let (child_input, child_name) = NodeName::parse(data_buf)?;
                    let (child_input, child_id) = PermanentId::parse(child_input)?;

                    children.insert(child_name, child_id);

                    data_buf = child_input;
                }

                let data = NodeData::Directory {
                    permissions,
                    children_size,
                    children,
                };

                (data_buf, data)
            }
            _ => unimplemented!(),
        };

        Ok((input, node_data))
    }

    pub fn stub_file(size: u64) -> Self {
        Self::File {
            permissions: FilePermissions::default(),
            content: FileContent::Stub { size },
            associated_data: HashMap::new(),
        }
    }
}
