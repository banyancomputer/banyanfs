use async_trait::async_trait;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::api::client::ApiRequest;

#[derive(Serialize)]
pub(crate) struct CurrentUsageLimit;

#[derive(Deserialize)]
pub struct CurrentUsageLimitResponse {
    soft_hot_storage_limit: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    hard_hot_storage_limit: Option<usize>,

    #[serde(rename = "size")]
    _size: usize,
}

impl CurrentUsageLimitResponse {
    pub fn hard_hot_storage_limit(&self) -> usize {
        self.hard_hot_storage_limit
            .unwrap_or(self.soft_hot_storage_limit)
    }

    pub fn soft_hot_storage_limit(&self) -> usize {
        self.soft_hot_storage_limit
    }
}

#[async_trait]
impl ApiRequest for CurrentUsageLimit {
    type Response = CurrentUsageLimitResponse;

    fn method(&self) -> Method {
        Method::GET
    }

    fn path(&self) -> String {
        "/api/v1/buckets/usage_limit".to_string()
    }
}
