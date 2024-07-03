use async_trait::async_trait;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiSnapshot};

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
    type Response = Vec<ApiSnapshot>;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/snapshots", self.drive_id)
    }
}

impl PlatformApiRequest for GetAllRequest {}
