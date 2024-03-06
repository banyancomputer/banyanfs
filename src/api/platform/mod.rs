pub mod requests;

mod api_drive;
mod api_drive_metadata;
mod drive_kind;
mod storage_class;

pub use api_drive::{ApiDrive, ApiDriveUpdateAttributes};
pub use api_drive_metadata::ApiDriveMetadata;
pub use drive_kind::DriveKind;
pub use storage_class::StorageClass;

pub type DriveId = String;

pub type DriveMetadataId = String;
