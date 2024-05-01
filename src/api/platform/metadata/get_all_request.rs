use async_trait::async_trait;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiMetadata};

pub(crate) struct GetAllRequest {
    drive_id: ApiDriveId,
}

impl GetAllRequest {
    pub(crate) fn new(drive_id: ApiDriveId) -> Self {
        GetAllRequest { drive_id }
    }
}

#[async_trait]
impl ApiRequest for GetAllRequest {
    type Response = ApiMetadata;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/metadata", self.drive_id)
    }
}

impl PlatformApiRequest for GetAllRequest {}
