use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiMetadata};

#[derive(Serialize)]
pub(crate) struct GetCurrentRequest {
    drive_id: ApiDriveId,
}

impl GetCurrentRequest {
    pub(crate) fn new(drive_id: ApiDriveId) -> Self {
        Self { drive_id }
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
