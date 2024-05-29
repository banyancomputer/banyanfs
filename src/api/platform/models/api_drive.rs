use crate::api::platform::{DriveKind, StorageClass};
use serde::{Deserialize, Serialize};
pub type ApiDriveId = String;

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiDrive {
    pub id: ApiDriveId,
    pub name: String,

    #[serde(rename = "type")]
    pub(crate) kind: DriveKind,

    pub(crate) storage_class: StorageClass,

    // The following two are always present except for during bucket creation
    #[serde(rename = "owner_id", skip_serializing_if = "Option::is_none")]
    _owner_id: Option<String>,

    #[serde(rename = "updated_at", skip_serializing_if = "Option::is_none")]
    _updated_at: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct ApiDriveUpdateAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
