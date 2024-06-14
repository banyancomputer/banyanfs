use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiMetadata};

#[derive(Serialize)]
pub(crate) struct GetAllRequest {
    drive_id: ApiDriveId,
}

impl GetAllRequest {
    pub(crate) fn new(drive_id: ApiDriveId) -> Self {
        Self { drive_id }
    }
}

#[async_trait]
impl ApiRequest for GetAllRequest {
    type Response = Vec<ApiMetadata>;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/metadata", self.drive_id)
    }
}

impl PlatformApiRequest for GetAllRequest {}
