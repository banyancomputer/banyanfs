use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiUserKey {
    id: ApiKeyId,
    name: String,
    user_id: UserId,
    api_access: bool,
    pem: String,
    fingerprint: String,
}

impl ApiUserKey {
    /// Key Id
    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }

    /// Name of the Key
    pub fn name(&self) -> &String {
        &self.name
    }

    /// User Id of the Owner of the Key
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// API usability
    pub fn api_access(&self) -> bool {
        self.api_access
    }

    /// PEM
    pub fn pem(&self) -> &str {
        &self.pem
    }

    /// Public Key Fingerprint
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

pub type ApiKeyId = String;
pub type UserId = String;
