use serde::{Deserialize, Serialize};

use crate::api::platform::{ApiDrive, DriveKind, StorageClass};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct TombBucket {
    id: String,
    name: String,

    #[serde(rename = "type")]
    kind: DriveKind,

    storage_class: StorageClass,
}

impl From<ApiDrive> for TombBucket {
    fn from(api_drive: ApiDrive) -> Self {
        Self {
            id: api_drive.id,
            name: api_drive.name,
            kind: api_drive.kind,
            storage_class: api_drive.storage_class,
        }
    }
}
