use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiDriveKey {
    id: ApiDriveKeyId,
    fingerprint: String,
}

impl ApiDriveKey {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn id(&self) -> &ApiDriveKeyId {
        &self.id
    }
}

pub type ApiDriveKeyId = String;
