pub mod requests;

mod api_drive;
mod api_metadata;
mod api_snapshot;
mod drive_kind;
mod storage_class;

pub use api_drive::{ApiDrive, ApiDriveId, ApiDriveUpdateAttributes};
pub use api_metadata::{ApiMetadata, ApiMetadataId, ApiMetadataState};
pub use api_snapshot::{ApiSnapshot, ApiSnapshotId, ApiSnapshotState};
pub use drive_kind::DriveKind;
pub use storage_class::StorageClass;

pub type ApiKeyId = String;
