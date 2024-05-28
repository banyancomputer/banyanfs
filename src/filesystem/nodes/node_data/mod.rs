use std::collections::hash_map::Entry;
use std::collections::HashMap;

use futures::{AsyncWrite, AsyncWriteExt};
use winnow::binary::le_u16;
use winnow::Parser;

use crate::codec::filesystem::{DirectoryPermissions, FilePermissions};
use crate::codec::{Cid, ParserResult, PermanentId, Stream};
use crate::filesystem::nodes::{NodeKind, NodeName};
use crate::filesystem::FileContent;

mod child_map;
use child_map::ChildMap;

use self::child_map::ChildMapEntry;

pub enum NodeData {
    File {
        permissions: FilePermissions,
        associated_data: ChildMap,
        content: FileContent,
    },
    #[allow(dead_code)]
    AssociatedData { content: FileContent },
    Directory {
        permissions: DirectoryPermissions,
        children: ChildMap,
    },
}

impl NodeData {
    pub(crate) fn add_child(
        &mut self,
        name: NodeName,
        id: PermanentId,
        cid: Cid,
        size: u64,
    ) -> Result<(), NodeDataError> {
        let child_map = self.children_mut().ok_or(NodeDataError::NotAParent)?;
        match child_map.entry(name) {
            Entry::Occupied(_) => return Err(NodeDataError::ChildNameExists),
            Entry::Vacant(entry) => {
                entry.insert(ChildMapEntry::new(id, cid, size));
            }
        }
        Ok(())
    }

    pub fn update_child(
        &mut self,
        child_permanent_id: &PermanentId,
        cid: Cid,
        size: u64,
    ) -> Result<(), NodeDataError> {
        let children = match self.children_mut() {
            None => return Ok(()),
            Some(children) => children,
        };

        let child = children
            .iter_mut()
            .find(|entry| entry.1.permanent_id() == child_permanent_id)
            .ok_or(NodeDataError::ChildIdMissing)?
            .1;
        child.set_cid(cid);
        child.set_size(size);
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
                children,
            } => {
                written_bytes += permissions.encode(writer).await?;
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
            children: HashMap::new(),
        }
    }

    pub(crate) fn ordered_child_pids(&self) -> Vec<PermanentId> {
        let mut child_pairs = self
            .children()
            .map(|child_map| child_map.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        child_pairs.sort_by(|(_, a), (_, b)| a.permanent_id().cmp(b.permanent_id()));
        child_pairs
            .into_iter()
            .map(|(_, id)| *id.permanent_id())
            .collect()
    }

    fn children(&self) -> Option<&ChildMap> {
        match self {
            Self::AssociatedData { .. } => None,
            Self::Directory { children, .. } => children.into(),
            Self::File {
                associated_data, ..
            } => associated_data.into(),
        }
    }
    fn children_mut(&mut self) -> Option<&mut ChildMap> {
        match self {
            Self::AssociatedData { .. } => None,
            Self::Directory { children, .. } => children.into(),
            Self::File {
                associated_data, ..
            } => associated_data.into(),
        }
    }
    pub(crate) fn data_cids(&self) -> Option<Vec<Cid>> {
        match self {
            NodeData::File { content, .. } | NodeData::AssociatedData { content } => {
                content.data_cids()
            }
            _ => None,
        }
    }

    pub(crate) fn parse(input: Stream) -> ParserResult<Self> {
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
                let (data_buf, children) = parse_children(data_buf)?;

                let data = NodeData::Directory {
                    permissions,
                    children,
                };

                Ok((data_buf, data))
            }
            _ => unimplemented!(),
        }
    }

    pub(crate) fn remove_child(&mut self, name: &NodeName) -> Result<PermanentId, NodeDataError> {
        let child_map = self.children_mut().ok_or(NodeDataError::NotAParent)?;
        match child_map.remove(name) {
            Some(id) => Ok(*id.permanent_id()),
            None => Err(NodeDataError::ChildNameMissing),
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
        let child_map = self.children_mut().ok_or(NodeDataError::NotAParent)?;
        child_map.retain(|_, id| id.permanent_id() != permanent_id);
        Ok(())
    }

    pub(crate) fn size(&self) -> u64 {
        tracing::warn!("size of child entries are not yet included in parents");

        match self {
            NodeData::AssociatedData { content } => content.size(),
            NodeData::Directory { .. } => {
                let base_size = DirectoryPermissions::size() + 8;
                base_size as u64 + self.children_size()
            }
            NodeData::File { content, .. } => {
                let base_size = FilePermissions::size();

                base_size as u64 + content.size() + self.children_size()
            }
        }
    }

    fn children_size(&self) -> u64 {
        let children = match self.children() {
            None => return 0,
            Some(children) => children,
        };
        children.iter().fold(0, |acc, (name, entry)| {
            acc + name.size() as u64 + std::mem::size_of::<ChildMapEntry>() as u64 + entry.size()
        })
    }

    pub(crate) fn empty_file() -> Self {
        Self::File {
            permissions: FilePermissions::default(),
            associated_data: HashMap::new(),
            content: FileContent::EmptyFile,
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
    children: &ChildMap,
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
    children.sort_by(|(_, a), (_, b)| a.permanent_id().cmp(b.permanent_id()));

    for (name, id) in children {
        written_bytes += name.encode(writer).await?;
        written_bytes += id.encode(writer).await?;
    }

    Ok(written_bytes)
}

fn parse_children(input: Stream) -> ParserResult<ChildMap> {
    let (data_buf, children_count) = le_u16.parse_peek(input)?;

    let mut children = HashMap::new();
    let mut child_buf = data_buf;

    for _ in 0..children_count {
        let (remaining, child_name) = NodeName::parse(child_buf)?;
        let (remaining, child_entry) = ChildMapEntry::parse(remaining)?;

        children.insert(child_name, child_entry);

        child_buf = remaining;
    }

    Ok((child_buf, children))
}

#[derive(Debug, thiserror::Error)]
pub enum NodeDataError {
    #[error("attempted to add child with a name that was already present")]
    ChildNameExists,

    #[error("attempted to remove a child that was not present")]
    ChildNameMissing,

    #[error("non-parent node cannot have or interact with children")]
    NotAParent,

    #[error("Passed in PermanentId does not refer to a valid child")]
    ChildIdMissing,
}
