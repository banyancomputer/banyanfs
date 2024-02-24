use std::collections::{HashMap, HashSet};

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

type EncodedAssociation = (usize, Option<Vec<PermanentId>>, Option<Vec<Cid>>);

impl NodeData {
    #[tracing::instrument(skip(self, rng, writer))]
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        data_key: Option<&AccessKey>,
    ) -> std::io::Result<EncodedAssociation> {
        let mut written_bytes = 0;

        written_bytes += self.kind().encode(writer).await?;
        tracing::trace!(kind = ?self.kind(), encode_len = written_bytes, "kind");

        match &self {
            NodeData::File {
                permissions,
                content,
                associated_data,
            } => {
                let perm_len = permissions.encode(writer).await?;
                tracing::trace!(encoded_len = perm_len, "permissions");

                let (content_len, data_cids) = content.encode(rng, writer, data_key).await?;
                tracing::trace!(
                    encoded_len = content_len,
                    data_cid_count = ?data_cids.as_ref().map(|c| c.len()).ok_or(0),
                    "data_cids"
                );

                written_bytes += content_len + perm_len;

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
                let perm_len = permissions.encode(writer).await?;
                written_bytes += perm_len;

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
                    written_bytes += id.encode(writer).await?;

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
    ) -> ParserResult<'a, (Self, Vec<PermanentId>)> {
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

                (data_buf, (data, Vec::new()))
            }
            //NodeKind::AssociatedData => {}
            NodeKind::Directory => {
                let (data_buf, permissions) = DirectoryPermissions::parse(input)?;
                let perm_len = input.len() - data_buf.len();
                tracing::trace!(bytes_read = ?perm_len, "directory_permissions");

                let (data_buf, children_size) = le_u64(data_buf)?;
                let children_size_len = input.len() - data_buf.len() - perm_len;
                tracing::trace!(bytes_read = ?children_size_len, "children_size");

                let (data_buf, children_count) = le_u16(data_buf)?;
                let children_count_len =
                    input.len() - data_buf.len() - perm_len - children_size_len;
                tracing::trace!(bytes_read = ?children_count_len, children_count, "children_count");

                let mut desired_nodes = HashSet::new();

                let mut children = HashMap::new();
                let mut child_buf = data_buf;
                for idx in 0..children_count {
                    let _guard = tracing::trace_span!("child", child_idx = idx, remaining_buf = ?child_buf.len()).entered();

                    let (remaining, child_name) = NodeName::parse(child_buf)?;
                    let name_len = child_buf.len() - remaining.len();
                    tracing::trace!(?child_name, bytes_read = name_len, "child_name");

                    let (remaining, child_id) = PermanentId::parse(remaining)?;
                    let id_len = child_buf.len() - remaining.len() - name_len;
                    tracing::trace!(child_perm_id = ?child_id, bytes_read = id_len, "child_perm_id");

                    children.insert(child_name, child_id);
                    desired_nodes.insert(child_id);

                    let bytes_read = child_buf.len() - remaining.len();
                    tracing::trace!(bytes_read, "complete");
                    child_buf = remaining;
                }

                let desired_node_ids = desired_nodes.into_iter().collect::<Vec<_>>();
                let data = NodeData::Directory {
                    permissions,
                    children_size,
                    children,
                };

                (child_buf, (data, desired_node_ids))
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
