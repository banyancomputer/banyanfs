use async_trait::async_trait;
use reqwest::{Method, RequestBuilder};
use serde::Serialize;

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
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

#[async_trait(?Send)]
impl ApiRequest for UpdateRequest {
    type Response = ();

    const METHOD: Method = Method::PUT;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(&self))
    }

    fn path(&self) -> String {
        format!("api/v1/buckets/{}", self.id)
    }
}

impl PlatformApiRequest for UpdateRequest {}
