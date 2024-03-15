use async_trait::async_trait;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiMetadata, ApiMetadataId};

pub(crate) struct GetRequest {
    drive_id: ApiDriveId,
    metadata_id: ApiMetadataId,
}

impl GetRequest {
    pub(crate) fn new(drive_id: ApiDriveId, metadata_id: ApiMetadataId) -> Self {
        GetRequest {
            drive_id,
            metadata_id,
        }
    }
}

#[async_trait]
impl ApiRequest for GetRequest {
    type Response = ApiMetadata;

    fn path(&self) -> String {
        format!(
            "/api/v1/buckets/{}/metadata/{}",
            self.drive_id, self.metadata_id
        )
    }
}

impl PlatformApiRequest for GetRequest {}
