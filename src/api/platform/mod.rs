pub mod requests;

mod api_drive;
mod drive_kind;
mod storage_class;

pub use api_drive::ApiDrive;
pub use drive_kind::DriveKind;
pub use storage_class::StorageClass;

pub type DriveId = String;
