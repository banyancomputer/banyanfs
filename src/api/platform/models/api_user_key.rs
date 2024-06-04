use serde::Deserialize;
/// User Key struct handed to us by the API
#[derive(Clone, Debug, Deserialize)]
pub struct ApiUserKey {
    id: ApiKeyId,
    name: String,
    user_id: ApiUserId,
    api_access: bool,
    public_key: String,
    fingerprint: String,
    created_at: String,
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
    pub fn user_id(&self) -> &ApiUserId {
        &self.user_id
    }

    /// API usability
    pub fn api_access(&self) -> bool {
        self.api_access
    }

    /// Public Key PEM
    pub fn public_key(&self) -> &str {
        &self.public_key
    }

    /// Public Key Fingerprint
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Created at time
    pub fn created_at(&self) -> &str {
        &self.created_at
    }
}

pub type ApiKeyId = String;
pub type ApiUserId = String;