use async_trait::async_trait;
use reqwest::Body;

use crate::api::client::{ApiRequest, DirectResponse, PlatformApiRequest};
use crate::api::platform::ApiDriveId;

pub(crate) struct PushRequest {
    drive_id: ApiDriveId,

    stream_body: Body,
}

impl PushRequest {
    pub(crate) fn new(drive_id: ApiDriveId, stream_body: impl Into<Body>) -> Self {
        let stream_body = stream_body.into();

        Self {
            drive_id,
            stream_body,
        }
    }
}

#[async_trait]
impl ApiRequest for PushRequest {
    type Response = DirectResponse;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/metadata", self.drive_id,)
    }
}

impl PlatformApiRequest for PushRequest {}
