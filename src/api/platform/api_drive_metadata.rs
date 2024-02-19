use serde::{Deserialize, Serialize};

use crate::api::platform::{DriveId, DriveKind, DriveMetadataId, StorageClass};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiDriveMetadata {
    pub id: DriveId,
    pub drive_metadata_id: DriveMetadataId,
}
