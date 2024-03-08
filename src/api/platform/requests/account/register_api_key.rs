use async_trait::async_trait;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiKeyId;
use crate::codec::crypto::VerifyingKey;

#[derive(Serialize)]
pub struct RegisterApiKey {
    #[serde(skip)]
    fingerprint: String,

    public_key: String,
}

impl RegisterApiKey {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn new(public_key: &VerifyingKey) -> Self {
        let fingerprint = public_key.fingerprint().to_hex();
        let public_key = public_key.to_spki().expect("valid key to be encodable");

        Self {
            fingerprint,
            public_key,
        }
    }
}

#[async_trait]
impl ApiRequest for RegisterApiKey {
    type Response = RegisterApiKeyResponse;

    const IS_PAYLOAD: bool = true;

    const METHOD: Method = Method::POST;

    fn path(&self) -> String {
        "/api/v1/auth/device_api_key".to_string()
    }
}

impl PlatformApiRequest for RegisterApiKey {}

#[derive(Debug, Deserialize)]
pub struct RegisterApiKeyResponse {
    id: ApiKeyId,
    fingerprint: String,
}

impl RegisterApiKeyResponse {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }
}
