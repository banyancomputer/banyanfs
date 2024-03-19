use async_trait::async_trait;
use reqwest::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiMetadataId, ApiSnapshotId};
use crate::codec::Cid;

pub(crate) struct CreateRequest {
    drive_id: ApiDriveId,
    metadata_id: ApiMetadataId,
    cids: Vec<String>,
}

impl CreateRequest {
    pub(crate) fn new(
        drive_id: ApiDriveId,
        metadata_id: ApiMetadataId,
        cids: &[Cid],
    ) -> CreateRequest {
        CreateRequest {
            drive_id,
            metadata_id,
            cids: cids.iter().map(|c| c.as_base64url_multicodec()).collect(),
        }
    }
}

#[async_trait(?Send)]
impl ApiRequest for CreateRequest {
    type Response = CreateResponse;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        let inner = InnerRequest(self.cids.as_slice());
        Ok(request_builder.json(&inner))
    }

    fn path(&self) -> String {
        format!(
            "/api/v1/buckets/{}/metadata/{}/snapshot",
            self.drive_id, self.metadata_id
        )
    }
}

// todo(sstelfox): this api request should be a bit more structured
#[derive(Debug, Serialize)]
struct InnerRequest<'a>(&'a [String]);

impl PlatformApiRequest for CreateRequest {}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateResponse {
    id: ApiSnapshotId,
}

impl CreateResponse {
    pub(crate) fn snapshot_id(&self) -> ApiSnapshotId {
        self.id.clone()
    }
}
