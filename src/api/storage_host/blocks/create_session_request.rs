use async_trait::async_trait;
use reqwest::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::api::client::{ApiError, ApiRequest, StorageHostApiRequest};

#[derive(Serialize)]
pub(crate) struct CreateSessionRequest {
    metadata_id: String,

    #[serde(skip)]
    session_data_size: u64,
}

impl CreateSessionRequest {
    pub(crate) fn new(metadata_id: &str, session_data_size: u64) -> Self {
        Self {
            metadata_id: metadata_id.to_string(),
            session_data_size,
        }
    }
}

#[async_trait(?Send)]
impl ApiRequest for CreateSessionRequest {
    type Response = CreateSessionResponse;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        // todo: add session data size to content length header...
        Ok(request_builder.json(&self))
    }

    fn path(&self) -> String {
        "/api/v1/blocks/new".to_string()
    }
}

impl StorageHostApiRequest for CreateSessionRequest {}

#[derive(Debug, Deserialize)]
pub struct CreateSessionResponse {
    upload_id: String,
}

impl CreateSessionResponse {
    pub fn upload_id(&self) -> &str {
        &self.upload_id
    }
}
