#![allow(dead_code)]

use async_trait::async_trait;
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::api::client::ApiError;

#[async_trait(?Send)]
pub(crate) trait ApiRequest {
    type Response: FromReqwestResponse;

    const METHOD: Method = Method::GET;

    const REQUIRES_AUTH: bool = true;

    async fn add_payload(
        &self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder)
    }

    fn path(&self) -> String;
}

#[async_trait(?Send)]
pub(crate) trait FromReqwestResponse: Sized {
    async fn from_response(response: Response) -> Result<Option<Self>, ApiError>;
}

#[async_trait(?Send)]
impl<T> FromReqwestResponse for T
where
    T: DeserializeOwned + Sized,
{
    async fn from_response(response: reqwest::Response) -> Result<Option<Self>, ApiError> {
        let status = response.status();
        if status == StatusCode::NO_CONTENT {
            return Ok(None);
        }

        match response.json::<T>().await {
            Ok(resp) => Ok(Some(resp)),
            Err(err) => {
                tracing::error!("failed to parse response API: {}", err);

                Err(ApiError::Message {
                    status_code: status.as_u16(),
                    message: format!("failed to parse response: {err}"),
                })
            }
        }
    }
}

pub(crate) trait PlatformApiRequest: ApiRequest {}

pub(crate) trait StorageHostApiRequest: ApiRequest {}
