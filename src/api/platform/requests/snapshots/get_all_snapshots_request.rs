use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiSnapshot};

#[derive(Serialize)]
pub(crate) struct GetAllSnapshotsRequest {
    drive_id: ApiDriveId,
}

impl GetAllSnapshotsRequest {
    pub(crate) fn new(drive_id: ApiDriveId) -> Self {
        Self { drive_id }
    }
}

#[async_trait]
impl ApiRequest for GetAllSnapshotsRequest {
    type Response = Vec<ApiSnapshot>;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/snapshots", self.drive_id)
    }
}

impl PlatformApiRequest for GetAllSnapshotsRequest {}
