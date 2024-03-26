use async_trait::async_trait;
use serde::Deserialize;

use crate::api::client::{ApiRequest, StorageHostApiRequest};

pub(crate) struct WhoAmIRequest;

#[async_trait(?Send)]
impl ApiRequest for WhoAmIRequest {
    type Response = WhoAmIResponse;

    fn path(&self) -> String {
        "/api/v1/who_am_i".to_string()
    }
}

impl StorageHostApiRequest for WhoAmIRequest {}

#[derive(Debug, Deserialize)]
pub struct WhoAmIResponse {
    consumed_storage: u64,
    fingerprint: String,
    platform_id: String,
}

impl WhoAmIResponse {
    pub fn consumed_storage(&self) -> u64 {
        self.consumed_storage
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn platform_id(&self) -> &str {
        &self.platform_id
    }
}
