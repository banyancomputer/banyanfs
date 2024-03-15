use serde::Deserialize;

use crate::api::platform::ApiDriveId;

#[derive(Debug, Deserialize)]
pub struct ApiDriveKey {
    id: ApiDriveKeyId,
    drive_id: ApiDriveId,

    fingerprint: String,

    #[serde(rename = "pem")]
    public_key: String,

    approved: bool,
}

impl ApiDriveKey {
    pub fn approved(&self) -> bool {
        self.approved
    }

    pub fn drive_id(&self) -> &ApiDriveKeyId {
        &self.drive_id
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn id(&self) -> &ApiDriveKeyId {
        &self.id
    }

    pub fn public_key(&self) -> &str {
        &self.public_key
    }
}

pub type ApiDriveKeyId = String;
