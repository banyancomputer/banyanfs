use async_trait::async_trait;
use serde::Deserialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};

pub(crate) struct GetStorageGrant {
    storage_domain: String,
}

impl GetStorageGrant {
    pub(crate) fn new(storage_domain: String) -> Self {
        Self { storage_domain }
    }
}

#[async_trait]
impl ApiRequest for GetStorageGrant {
    type Response = GetStorageGrantResponse;

    fn path(&self) -> String {
        format!("/api/v1/auth/storage_grant/{}", self.storage_domain)
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
