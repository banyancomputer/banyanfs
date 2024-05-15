use super::{ApiDriveId, ApiKeyId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiDriveAccess {
    user_key_id: ApiKeyId,
    #[serde(rename = "bucket_id")]
    drive_id: ApiDriveId,
    fingerprint: String,
    approved: bool,
}

impl ApiDriveAccess {
    pub fn approved(&self) -> bool {
        self.approved
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
