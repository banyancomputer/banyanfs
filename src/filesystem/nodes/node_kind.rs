use std::collections::HashMap;

use futures::AsyncWrite;

use crate::codec::crypto::AccessKey;
use crate::codec::filesystem::{DirectoryPermissions, FilePermissions};
use crate::codec::meta::PermanentId;
use crate::codec::ParserResult;
use crate::filesystem::nodes::NodeName;
use crate::filesystem::FileContent;

pub enum NodeKind {
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

impl NodeKind {
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        _writer: &mut W,
        _data_key: &AccessKey,
    ) -> std::io::Result<(usize, Vec<PermanentId>)> {
        todo!()
    }

    pub fn new_directory() -> Self {
        NodeKind::Directory {
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
        NodeKind::File {
            permissions: FilePermissions::default(),
            content: FileContent::Stub { size },
        }
    }
}
