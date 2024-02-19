use serde::{Deserialize, Serialize};

use crate::api::platform::{DriveId, DriveKind, StorageClass};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiDrive {
    pub(crate) id: DriveId,
    pub(crate) name: String,

    #[serde(rename = "type")]
    pub(crate) kind: DriveKind,

    pub(crate) storage_class: StorageClass,
}
