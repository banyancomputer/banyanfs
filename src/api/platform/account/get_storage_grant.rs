use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use reqwest::Url;
use serde::Deserialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};

pub(crate) struct GetStorageGrant {
    base_url: Url,
}

impl GetStorageGrant {
    pub(crate) fn new(base_url: Url) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl ApiRequest for GetStorageGrant {
    type Response = GetStorageGrantResponse;

    fn path(&self) -> String {
        let encoded_base_url = URL_SAFE.encode(self.base_url.as_str());
        format!("/api/v1/users/storage_grant/{}", encoded_base_url)
    }
}

impl PlatformApiRequest for GetStorageGrant {}

#[derive(Debug, Deserialize)]
pub struct GetStorageGrantResponse {
    authorization_token: String,
}

impl GetStorageGrantResponse {
    pub fn authorization_token(&self) -> &str {
        &self.authorization_token
    }
}
