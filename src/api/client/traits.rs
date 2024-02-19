#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use async_trait::async_trait;
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::api::client::ApiError;

#[async_trait]
pub(crate) trait ApiRequest: Serialize {
    type Response: DeserializeOwned + Sized;

    fn method(&self) -> Method {
        Method::GET
    }

    fn path(&self) -> String;

    fn is_payload(&self) -> bool {
        false
    }

    fn requires_auth(&self) -> bool {
        true
    }
}

//#[async_trait]
//pub(crate) trait ApiResponse: DeserializeOwned + Sized {
//    async fn from_response(response: reqwest::Response) -> Result<Self, ApiError>;
//}

//#[async_trait]
//impl<T: DeserializeOwned> ApiResponse for T {
//    async fn from_response(response: reqwest::Response) -> Result<Self, ApiError> {
//        response.json().await.map_err(ApiError::from)
//    }
//}
