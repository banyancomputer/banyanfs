use serde::Deserialize;

// todo(sstelfox): This should return the drive IDs its a member of as well
#[derive(Debug, Deserialize)]
pub struct ApiDriveKey {
    id: ApiDriveKeyId,

    fingerprint: String,

    #[serde(rename = "pem")]
    public_key: String,

    approved: bool,
}

impl ApiDriveKey {
    pub fn approved(&self) -> bool {
        self.approved
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
