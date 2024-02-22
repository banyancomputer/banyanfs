use std::collections::HashMap;

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};

use crate::codec::crypto::AccessKey;
use crate::codec::filesystem::{DirectoryPermissions, FilePermissions};
use crate::codec::{Cid, ParserResult, PermanentId};
use crate::filesystem::nodes::{NodeKind, NodeName};
use crate::filesystem::FileContent;

pub enum NodeData {
    File {
        permissions: FilePermissions,
        content: FileContent,
    },
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
            } => {
                written_bytes += permissions.encode(writer).await?;
                let (n, data_cids) = content.encode(rng, writer, data_key).await?;
                written_bytes += n;

                Ok((written_bytes, None, data_cids))
            }
            NodeData::Directory {
                permissions,
                children,
                children_size,
            } => {
                written_bytes += permissions.encode(writer).await?;

                let children_size_bytes = children_size.to_le_bytes();
                writer.write_all(&children_size_bytes).await?;
                written_bytes += children_size_bytes.len();

                let mut children = children.iter().collect::<Vec<_>>();
                children.sort_by(|(_, a), (_, b)| a.cmp(b));

                let mut children_ids = Vec::new();
                for (name, id) in children {
                    written_bytes += name.encode(writer).await?;
                    children_ids.push(*id);
                }

                Ok((written_bytes, Some(children_ids), None))
            }
        }
    }

    pub(crate) fn kind(&self) -> NodeKind {
        match self {
            NodeData::File { .. } => NodeKind::File,
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
        data_key: &AccessKey,
    ) -> ParserResult<'a, (Self, Vec<PermanentId>)> {
        todo!()
    }

    pub fn stub_file(size: u64) -> Self {
        Self::File {
            permissions: FilePermissions::default(),
            content: FileContent::Stub { size },
        }
    }
}
