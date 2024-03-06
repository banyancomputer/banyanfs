use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::{ApiRequest, DirectResponse, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiMetadataId};

#[derive(Serialize)]
pub(crate) struct PullRequest {
    drive_id: ApiDriveId,
    metadata_id: ApiMetadataId,
}

impl PullRequest {
    pub(crate) fn new(drive_id: ApiDriveId, metadata_id: ApiMetadataId) -> Self {
        Self {
            drive_id,
            metadata_id,
        }
    }
}

#[async_trait]
impl ApiRequest for PullRequest {
    type Response = DirectResponse;

    fn path(&self) -> String {
        format!(
            "/api/v1/buckets/{}/metadata/{}/pull",
            self.drive_id, self.metadata_id
        )
    }
}

impl PlatformApiRequest for PullRequest {}
