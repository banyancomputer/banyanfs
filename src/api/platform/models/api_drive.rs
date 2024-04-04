use serde::{Deserialize, Serialize};

use crate::api::platform::{DriveKind, StorageClass};

pub type ApiDriveId = String;

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiDrive {
    pub(crate) id: ApiDriveId,
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
    #[serde(rename = "access", skip_serializing_if = "Option::is_none")]
    _access: Option<InitialBucketAccess>,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
struct InitialBucketAccess {
    user_key_id: String,
    fingerprint: String,
    state: BucketAccessState,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BucketAccessState {
    Pending,
    Approved,
    Revoked,
}

#[derive(Debug, Default, Serialize)]
pub struct ApiDriveUpdateAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
}
