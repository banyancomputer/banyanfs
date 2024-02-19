use async_trait::async_trait;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::api::client::ApiRequest;

#[derive(Serialize)]
pub(crate) struct CurrentUsage;

#[derive(Deserialize)]
pub struct CurrentUsageResponse {
    pub(crate) data_size: i64,
    pub(crate) meta_size: i64,
}

impl CurrentUsageResponse {
    pub fn total_size(&self) -> i64 {
        self.data_size + self.meta_size
    }
}

#[async_trait]
impl ApiRequest for CurrentUsage {
    type Response = CurrentUsageResponse;

    fn method(&self) -> Method {
        Method::GET
    }

    fn path(&self) -> String {
        "/api/v1/buckets/usage".to_string()
    }
}
