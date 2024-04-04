use std::collections::hash_map::Entry;
use std::collections::HashMap;

use futures::{AsyncWrite, AsyncWriteExt};
use winnow::number::streaming::{le_u16, le_u64};

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
    AssociatedData {
        content: FileContent,
    },
    Directory {
        permissions: DirectoryPermissions,
        children_size: u64,
        children: HashMap<NodeName, PermanentId>,
    },
}

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

    #[tracing::instrument(skip(self, writer))]
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += self.kind().encode(writer).await?;
        tracing::trace!(kind = ?self.kind(), encode_len = written_bytes, "kind");

        match &self {
            NodeData::Directory {
                permissions,
                children_size,
                children,
            } => {
                written_bytes += permissions.encode(writer).await?;

                let size_bytes = children_size.to_le_bytes();
                writer.write_all(&size_bytes).await?;
                written_bytes += size_bytes.len();

                written_bytes += encode_children(children, writer).await?;

                Ok(written_bytes)
            }
            NodeData::File {
                permissions,
                associated_data,
                content,
            } => {
                written_bytes += permissions.encode(writer).await?;
                written_bytes += encode_children(associated_data, writer).await?;
                written_bytes += content.encode(writer).await?;

                Ok(written_bytes)
            }
            _ => unimplemented!(),
        }
    }

    pub(crate) fn kind(&self) -> NodeKind {
        match self {
            NodeData::File { .. } => NodeKind::File,
            NodeData::AssociatedData { .. } => NodeKind::AssociatedData,
            NodeData::Directory { .. } => NodeKind::Directory,
        }
    }

    pub(crate) fn new_directory() -> Self {
        Self::Directory {
            permissions: DirectoryPermissions::default(),
            children_size: 0,
            children: HashMap::new(),
        }
    }

    pub(crate) fn ordered_child_pids(&self) -> Vec<PermanentId> {
        let children = match self {
            NodeData::File {
                associated_data, ..
            } => associated_data,
            NodeData::Directory { children, .. } => children,
            _ => return Vec::new(),
        };

        let mut child_pairs = children.iter().collect::<Vec<_>>();
        child_pairs.sort_by(|(_, a), (_, b)| a.cmp(b));
        child_pairs.into_iter().map(|(_, id)| *id).collect()
    }

    pub(crate) fn data_cids(&self) -> Option<Vec<Cid>> {
        match self {
            NodeData::File { content, .. } | NodeData::AssociatedData { content } => {
                content.data_cids()
            }
            _ => None,
        }
    }

    pub(crate) fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, kind) = NodeKind::parse(input)?;

        match kind {
            NodeKind::File => {
                let (data_buf, permissions) = FilePermissions::parse(input)?;
                let (data_buf, associated_data) = parse_children(data_buf)?;
                let (data_buf, content) = FileContent::parse(data_buf)?;

                let data = NodeData::File {
                    permissions,
                    associated_data,
                    content,
                };

                Ok((data_buf, data))
            }
            //NodeKind::AssociatedData => {}
            NodeKind::Directory => {
                let (data_buf, permissions) = DirectoryPermissions::parse(input)?;
                let (data_buf, children_size) = le_u64(data_buf)?;
                let (data_buf, children) = parse_children(data_buf)?;

                let data = NodeData::Directory {
                    permissions,
                    children_size,
                    children,
                };

                Ok((data_buf, data))
            }
            _ => unimplemented!(),
        }
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

    pub(crate) fn size(&self) -> u64 {
        tracing::warn!("size of child entries are not yet included in parents");

        match self {
            NodeData::AssociatedData { content } => content.size(),
            NodeData::Directory {
                children,
                children_size,
                ..
            } => {
                let base_size = DirectoryPermissions::size() + 8;

                let child_map_size = children
                    .iter()
                    .fold(0, |acc, (name, _)| acc + name.size() + PermanentId::size());

                (base_size + child_map_size) as u64 + children_size
            }
            NodeData::File {
                associated_data,
                content,
                ..
            } => {
                let base_size = FilePermissions::size();

                let associated_data_size = associated_data
                    .iter()
                    .fold(0, |acc, (name, _)| acc + name.size() + PermanentId::size());

                (base_size + associated_data_size) as u64 + content.size()
            }
        }
    }

    pub(crate) fn full_file(content: FileContent) -> Self {
        Self::File {
            permissions: FilePermissions::default(),
            associated_data: HashMap::new(),
            content,
        }
    }

    pub(crate) fn stub_file(data_size: u64) -> Self {
        Self::File {
            permissions: FilePermissions::default(),
            associated_data: HashMap::new(),
            content: FileContent::Stub { data_size },
        }
    }
}

async fn encode_children<W: AsyncWrite + Unpin + Send>(
    children: &HashMap<NodeName, PermanentId>,
    writer: &mut W,
) -> std::io::Result<usize> {
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

    for (name, id) in children {
        written_bytes += name.encode(writer).await?;
        written_bytes += id.encode(writer).await?;
    }

    Ok(written_bytes)
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
