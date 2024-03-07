use async_trait::async_trait;
use reqwest::Method;
use serde::Serialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiDriveUpdateAttributes};

#[derive(Serialize)]
pub(crate) struct UpdateRequest {
    id: ApiDriveId,
    attrs: ApiDriveUpdateAttributes,
}

impl UpdateRequest {
    pub fn new(id: ApiDriveId, attrs: ApiDriveUpdateAttributes) -> Self {
        UpdateRequest { id, attrs }
    }
}

#[async_trait]
impl ApiRequest for UpdateRequest {
    type Response = ();

    fn method(&self) -> Method {
        Method::PUT
    }

    fn path(&self) -> String {
        format!("api/v1/buckets/{}", self.id)
    }

    fn is_payload(&self) -> bool {
        true
    }
}

impl PlatformApiRequest for UpdateRequest {}