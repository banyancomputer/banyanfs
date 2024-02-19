use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::ApiRequest;
use crate::api::platform::ApiDriveMetadata;

#[derive(Serialize)]
pub(crate) struct PullCurrentRequest {
    drive_id: String,
}

impl PullCurrentRequest {
    pub(crate) fn new(drive_id: &str) -> Self {
        Self {
            drive_id: drive_id.to_string(),
        }
    }
}

#[async_trait]
impl ApiRequest for PullCurrentRequest {
    type Response = ApiDriveMetadata;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/metadata/pull", self.drive_id)
    }
}
