use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiDrive;

#[derive(Debug, Serialize)]
pub(crate) struct GetRequest {
    drive_id: String,
}

impl GetRequest {
    pub(crate) fn new(drive_id: String) -> Self {
        GetRequest { drive_id }
    }
}

#[async_trait]
impl ApiRequest for GetRequest {
    type Response = ApiDrive;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}", self.drive_id)
    }
}

impl PlatformApiRequest for GetRequest {}
