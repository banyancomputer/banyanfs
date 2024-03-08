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

    const IS_PAYLOAD: bool = true;

    const METHOD: Method = Method::PUT;

    fn path(&self) -> String {
        format!("api/v1/buckets/{}", self.id)
    }
}

impl PlatformApiRequest for UpdateRequest {}
