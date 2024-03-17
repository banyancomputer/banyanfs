use std::collections::hash_map::Entry;
use std::collections::HashMap;

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u16;

use crate::codec::crypto::AccessKey;
use crate::codec::filesystem::{DirectoryPermissions, FilePermissions};
use crate::codec::{Cid, ParserResult, PermanentId};
use crate::filesystem::nodes::{NodeKind, NodeName};
use crate::filesystem::FileContent;

pub enum NodeData {
    File {
        permissions: FilePermissions,
        associated_data: HashMap<NodeName, PermanentId>,
        content: FileContent,
    },
    AssoicatedData,
    Directory {
        permissions: DirectoryPermissions,
        children: HashMap<NodeName, PermanentId>,
    },
}

type EncodedAssociation = (usize, Vec<PermanentId>, Vec<Cid>);

impl NodeData {
    pub(crate) fn add_child(
        &mut self,
        name: NodeName,
        id: PermanentId,
    ) -> Result<(), NodeDataError> {
        let child_map = match self {
            NodeData::Directory { children, .. } => children,
            NodeData::File {
                associated_data, ..
            } => associated_data,
            _ => return Err(NodeDataError::NotAParent),
        };

        match child_map.entry(name) {
            Entry::Occupied(_) => return Err(NodeDataError::NameExists),
            Entry::Vacant(entry) => {
                entry.insert(id);
            }
        }

        Ok(())
    }

    pub(crate) fn remove_child(&mut self, name: &NodeName) -> Result<PermanentId, NodeDataError> {
        let child_map = match self {
            NodeData::Directory { children, .. } => children,
            NodeData::File {
                associated_data, ..
            } => associated_data,
            _ => return Err(NodeDataError::NotAParent),
        };

        match child_map.remove(name) {
            Some(id) => Ok(id),
            None => Err(NodeDataError::NameMissing),
        }
    }

    /// This function should be used with care, multiple entries may potentially be pointing to the
    /// same the same permanent ID and this will remove all of them. This is also signficantly less
    /// performant than removing a child by name as it requires visiting every possible entry in
    /// the allocated map under the hood.
    ///
    /// This function will not fail like others if there are no children matching the permanent ID.
    /// It will just complete successfully and quietly.
    pub(crate) fn remove_permanent_id(
        &mut self,
        permanent_id: &PermanentId,
    ) -> Result<(), NodeDataError> {
        let child_map = match self {
            NodeData::Directory { children, .. } => children,
            NodeData::File {
                associated_data, ..
            } => associated_data,
            _ => return Err(NodeDataError::NotAParent),
        };

        child_map.retain(|_, id| id != permanent_id);

        Ok(())
    }

    pub(crate) fn child_pids(&self) -> Option<Vec<PermanentId>> {
        let children = match self {
            NodeData::File {
                associated_data, ..
            } => associated_data,
            NodeData::Directory { children, .. } => children,
            _ => return None,
        };

        Some(children.values().cloned().collect())
    }

    pub(crate) fn data_cids(&self) -> Option<Vec<Cid>> {
        tracing::warn!("impl needed for returning data CIDs for file content");
        None
    }

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
                associated_data,
                content,
            } => {
                written_bytes += permissions.encode(writer).await?;

                let (child_len, ordered_ids) = encode_children(associated_data, writer).await?;
                written_bytes += child_len;

                let (content_len, data_cids) = content.encode(rng, writer, data_key).await?;
                written_bytes += content_len;

                Ok((written_bytes, ordered_ids, data_cids))
            }
            NodeData::Directory {
                permissions,
                children,
            } => {
                let perm_len = permissions.encode(writer).await?;
                written_bytes += perm_len;

                let (child_len, ordered_ids) = encode_children(children, writer).await?;
                written_bytes += child_len;

                Ok((written_bytes, ordered_ids, Vec::new()))
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
                let (data_buf, associated_data) = parse_children(data_buf)?;
                let (data_buf, content) = FileContent::parse(data_buf, data_key)?;

                let desired_node_ids = associated_data.values().cloned().collect::<Vec<_>>();

                let data = NodeData::File {
                    permissions,
                    associated_data,
                    content,
                };

                (data_buf, (data, desired_node_ids))
            }
            //NodeKind::AssociatedData => {}
            NodeKind::Directory => {
                let (data_buf, permissions) = DirectoryPermissions::parse(input)?;
                let (data_buf, children) = parse_children(data_buf)?;

                let desired_node_ids = children.values().cloned().collect::<Vec<_>>();

                let data = NodeData::Directory {
                    permissions,
                    children,
                };

                (data_buf, (data, desired_node_ids))
            }
            _ => unimplemented!(),
        };

        Ok((input, node_data))
    }

    pub fn stub_file(size: u64) -> Self {
        Self::File {
            permissions: FilePermissions::default(),
            associated_data: HashMap::new(),
            content: FileContent::Stub { size },
        }
    }
}

async fn encode_children<W: AsyncWrite + Unpin + Send>(
    children: &HashMap<NodeName, PermanentId>,
    writer: &mut W,
) -> std::io::Result<(usize, Vec<PermanentId>)> {
    let child_count = children.len();
    if child_count > u16::MAX as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "too many children in a single directory entry",
        ));
    }

    let child_count_bytes = (child_count as u16).to_le_bytes();
    writer.write_all(&child_count_bytes).await?;
    let mut written_bytes = child_count_bytes.len();

    let mut children = children.iter().collect::<Vec<_>>();
    children.sort_by(|(_, a), (_, b)| a.cmp(b));

    let mut ordered_child_ids = Vec::with_capacity(child_count);
    for (name, id) in children {
        written_bytes += name.encode(writer).await?;
        written_bytes += id.encode(writer).await?;
        ordered_child_ids.push(*id);
    }

    Ok((written_bytes, ordered_child_ids))
}

fn parse_children(input: &[u8]) -> ParserResult<HashMap<NodeName, PermanentId>> {
    let (data_buf, children_count) = le_u16(input)?;

    let mut children = HashMap::new();
    let mut child_buf = data_buf;

    for _ in 0..children_count {
        let (remaining, child_name) = NodeName::parse(child_buf)?;
        let (remaining, child_id) = PermanentId::parse(remaining)?;

        children.insert(child_name, child_id);

        child_buf = remaining;
    }

    Ok((child_buf, children))
}

#[derive(Debug, thiserror::Error)]
pub enum NodeDataError {
    #[error("attempted to add child with a name that was already present")]
    NameExists,

    #[error("attempted to remove a child that was not present")]
    NameMissing,

    #[error("non-parent node cannot have or interact with children")]
    NotAParent,
}
