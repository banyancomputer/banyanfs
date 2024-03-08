#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use async_trait::async_trait;
use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::api::client::ApiError;

// todo(sstelfox): The request itself shouldn't require the Serialize implmentation, I should
// support building a custom payload (and maybe an entire reqwest::Body for stream support). This
// is a lazy hack for speed right now.
#[async_trait]
pub(crate) trait ApiRequest: Serialize {
    type Response: FromReqwestResponse;

    const IS_PAYLOAD: bool = false;

    const METHOD: Method = Method::GET;

    const REQUIRES_AUTH: bool = true;

    fn path(&self) -> String;
}

#[async_trait(?Send)]
pub(crate) trait FromReqwestResponse: Sized {
    async fn from_response(response: reqwest::Response) -> Result<Option<Self>, ApiError>;
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
