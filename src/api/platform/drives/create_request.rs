use async_trait::async_trait;
use reqwest::{Method, RequestBuilder};
use serde::Serialize;

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDrive, DriveKind, StorageClass};

#[derive(Serialize)]
pub(crate) struct CreateRequest {
    pub(crate) name: String,

    #[serde(rename = "type")]
    pub(crate) kind: DriveKind,

    pub(crate) storage_class: StorageClass,

    #[serde(rename = "initial_bucket_key_pem")]
    pub(crate) owner_key: String,
}

#[async_trait(?Send)]
impl ApiRequest for CreateRequest {
    type Response = ApiDrive;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(&self))
    }

    fn path(&self) -> String {
        "/api/v1/buckets".to_string()
    }
}

impl PlatformApiRequest for CreateRequest {}
