use async_trait::async_trait;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiUserKeyAccess;

pub(crate) struct GetAllRequest {
    bucket_id: String,
}

impl GetAllRequest {
    pub fn new(bucket_id: String) -> Self {
        Self { bucket_id }
    }
}

#[async_trait]
impl ApiRequest for GetAllRequest {
    type Response = Vec<ApiUserKeyAccess>;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/access", self.bucket_id)
    }
}

impl PlatformApiRequest for GetAllRequest {}
