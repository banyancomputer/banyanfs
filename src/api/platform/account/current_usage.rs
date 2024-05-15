use async_trait::async_trait;
use serde::Deserialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};

pub(crate) struct CurrentUsage;

#[derive(Deserialize)]
pub struct CurrentUsageResponse {
    hot_storage: u64,
    archival_storage: u64,
}

impl CurrentUsageResponse {
    pub fn hot_usage(&self) -> u64 {
        self.hot_storage
    }

    pub fn archival_usage(&self) -> u64 {
        self.archival_storage
    }
}

#[async_trait]
impl ApiRequest for CurrentUsage {
    type Response = CurrentUsageResponse;

    fn path(&self) -> String {
        "/api/v1/buckets/usage".to_string()
    }
}

impl PlatformApiRequest for CurrentUsage {}
