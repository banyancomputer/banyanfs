mod actor_settings;
mod content_reference;
mod drive;
mod drive_access;
mod drive_loader;
mod file_content;
mod nodes;
mod vector_clock;

pub use actor_settings::ActorSettings;
pub use content_reference::ContentReference;
pub use drive::Drive;
pub use drive_access::DriveAccess;
pub use drive_loader::DriveLoader;
pub use file_content::FileContent;
pub use nodes::*;
pub use vector_clock::VectorClock;

#[derive(Debug)]
pub struct FilesystemEntry {
    //parent_id: Optioon<EntryId>,
    node: FilesystemNode,
}

#[derive(Debug)]
pub enum FilesystemNode {
    File(File),
    Directory(Directory),
}
