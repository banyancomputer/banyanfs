use async_trait::async_trait;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::api::client::{ApiRequest, PlatformApiRequest};

#[derive(Serialize)]
pub(crate) struct CurrentUsage;

#[derive(Deserialize)]
pub struct CurrentUsageResponse {
    data_size: usize,
    meta_size: usize,
}

impl CurrentUsageResponse {
    pub fn total_usage(&self) -> usize {
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

impl PlatformApiRequest for CurrentUsage {}
