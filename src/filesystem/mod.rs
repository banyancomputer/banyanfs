mod filesystem_id;

pub use filesystem_id::FilesystemId;

use crate::codec::crypto::SigningKey;

type ActorId = u16;

pub struct Drive {
    filesystem_id: FilesystemId,
    root: DriveDirectory,
}

impl Drive {
    pub fn initialize(_signing_key: &SigningKey) -> Self {
        let mut rng = crate::utils::crypto_rng();

        Self {
            filesystem_id: FilesystemId::generate(&mut rng),
            root: DriveDirectory::new(),
        }
    }
}

#[derive(Clone)]
pub(crate) enum DriveEntity {
    File(DriveFile),
    Directory(DriveDirectory),
}

#[derive(Clone)]
pub(crate) struct DriveFile {
    content_ref: u16,
}

#[derive(Clone)]
pub(crate) struct DriveDirectory;

impl DriveDirectory {
    pub(crate) fn new() -> Self {
        Self
    }
}
