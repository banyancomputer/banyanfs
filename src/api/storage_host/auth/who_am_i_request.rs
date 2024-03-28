use async_trait::async_trait;
use serde::Deserialize;

use crate::api::client::{ApiRequest, StorageHostApiRequest};

pub(crate) struct WhoAmIRequest;

#[async_trait(?Send)]
impl ApiRequest for WhoAmIRequest {
    type Response = WhoAmIResponse;

    fn path(&self) -> String {
        "/api/v1/auth/who_am_i".to_string()
    }
}

impl StorageHostApiRequest for WhoAmIRequest {}

#[derive(Debug, Deserialize)]
pub struct WhoAmIResponse {
    consumed_storage: u64,
    platform_id: String,
    remaining_storage: u64,
}

impl WhoAmIResponse {
    pub fn consumed_storage(&self) -> u64 {
        self.consumed_storage
    }

    pub fn platform_id(&self) -> &str {
        &self.platform_id
    }

    pub fn remaining_storage(&self) -> u64 {
        self.remaining_storage
    }
}
