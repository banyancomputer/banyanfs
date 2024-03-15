use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiKey {
    id: ApiKeyId,
    fingerprint: String,
}

impl ApiKey {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }
}

pub type ApiKeyId = String;
