use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiMetadata;

#[derive(Serialize)]
pub(crate) struct GetCurrentRequest {
    drive_id: String,
}

impl GetCurrentRequest {
    pub(crate) fn new(drive_id: &str) -> Self {
        Self {
            drive_id: drive_id.to_string(),
        }
    }
}

#[async_trait]
impl ApiRequest for GetCurrentRequest {
    type Response = ApiMetadata;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/metadata/current", self.drive_id)
    }
}

impl PlatformApiRequest for GetCurrentRequest {}
