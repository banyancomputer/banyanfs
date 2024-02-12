use crate::codec::crypto::SymLockedAccessKey;
use crate::filesystem::ContentReference;

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

    pub fn size(&self) -> u64 {
        match self {
            FileContent::Encrypted { content, .. } => content.iter().map(|c| c.size()).sum(),
            FileContent::Public { content } => content.iter().map(|c| c.size()).sum(),
            FileContent::Stub { size } => *size,
        }
    }
}
