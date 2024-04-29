use async_trait::async_trait;
use reqwest::{Method, RequestBuilder};
use serde::Serialize;

use crate::api::{
    client::{ApiRequest, PlatformApiRequest},
    ApiError,
};

#[derive(Serialize)]
pub(crate) struct RevokeRequest {
    #[serde(skip)]
    bucket_id: String,
    fingerprint: String,
}

impl RevokeRequest {
    pub fn new(bucket_id: String, fingerprint: String) -> Self {
        Self {
            bucket_id,
            fingerprint,
        }
    }
}

#[async_trait(?Send)]
impl ApiRequest for RevokeRequest {
    type Response = ();
    const METHOD: Method = Method::DELETE;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(&self))
    }

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/access", self.bucket_id)
    }
}

impl PlatformApiRequest for RevokeRequest {}
