use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use super::ApiKeyId;

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiDriveAccess {
    user_key_id: ApiKeyId,
    #[serde(rename = "bucket_id")]
    drive_id: String,
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

impl Display for BucketAccessState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BucketAccessState::Pending => f.write_str("pending"),
            BucketAccessState::Approved => f.write_str("approved"),
            BucketAccessState::Revoked => f.write_str("revoked"),
        }
    }
}

impl ApiDriveAccess {
    pub fn state(&self) -> String {
        self.state.to_string()
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn user_key_id(&self) -> &str {
        &self.user_key_id
    }

    pub fn drive_id(&self) -> &str {
        &self.drive_id
    }
}
