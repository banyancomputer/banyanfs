mod filesystem_id;

pub use filesystem_id::FilesystemId;

pub struct Drive {
    filesystem_id: FilesystemId,
}
