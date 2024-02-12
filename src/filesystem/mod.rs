mod content_reference;
mod file_content;
mod nodes;

pub use content_reference::ContentReference;
pub use file_content::FileContent;
pub use nodes::*;

use crate::codec::crypto::SigningKey;
use crate::codec::FilesystemId;

pub struct Drive {
    _filesystem_id: FilesystemId,
    _root: DriveDirectory,
}

impl Drive {
    pub fn initialize(_signing_key: &SigningKey) -> Self {
        let mut rng = crate::utils::crypto_rng();

        Self {
            _filesystem_id: FilesystemId::generate(&mut rng),
            _root: DriveDirectory::new(),
        }
    }
}

#[derive(Clone)]
pub enum DriveEntity {
    File(DriveFile),
    Directory(DriveDirectory),
}

#[derive(Clone)]
pub struct DriveFile {
    _content_ref: u16,
}

#[derive(Clone)]
pub struct DriveDirectory;

impl DriveDirectory {
    pub(crate) fn new() -> Self {
        Self
    }
}
