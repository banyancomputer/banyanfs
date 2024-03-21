use async_trait::async_trait;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiMetadataId};

// todo(sstelfox): the ID here is currently for metadata but it really should be for the snapshot
// that is getting restored.

#[derive(Serialize)]
pub(crate) struct RestoreRequest {
    drive_id: ApiDriveId,
    metadata_id: ApiMetadataId,
}

impl RestoreRequest {
    pub(crate) fn new(drive_id: ApiDriveId, metadata_id: ApiMetadataId) -> Self {
        Self {
            drive_id,
            metadata_id,
        }
    }
}

#[async_trait]
impl ApiRequest for RestoreRequest {
    type Response = RestoreResponse;

    const METHOD: Method = Method::POST;

    fn path(&self) -> String {
        format!(
            "/api/v1/buckets/{}/snapshots/{}/restore",
            self.drive_id, self.metadata_id
        )
    }
}

impl PlatformApiRequest for RestoreRequest {}

// note(sstelfox): this response is meaningless and should be a 204 response
#[derive(Debug, Deserialize)]
pub(crate) struct RestoreResponse {
    #[serde(rename = "metadata_id")]
    _metadata_id: ApiMetadataId,
}
