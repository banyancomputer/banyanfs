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

    // The following two are always present except for during bucket creation
    #[serde(rename = "owner_id", skip_serializing_if = "Option::is_none")]
    _owner_id: Option<String>,

    #[serde(rename = "updated_at", skip_serializing_if = "Option::is_none")]
    _updated_at: Option<String>,

    // Only present in the response to bucket creation
    #[serde(rename = "initial_bucket_key", skip_serializing_if = "Option::is_none")]
    _unused_key: Option<InitialBucketKey>,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
struct InitialBucketKey {
    id: String,
    approved: bool,
    fingerprint: String,
}
