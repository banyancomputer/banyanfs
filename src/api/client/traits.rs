#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::api::client::ApiError;

pub(crate) trait ApiRequest {
    type Response: ApiResponse;
    type Payload: Serialize;

    fn method(&self) -> Method;

    fn path(&self) -> &str;

    fn payload(&self) -> Option<Self::Payload>;

    fn requires_auth(&self) -> bool {
        true
    }
}

pub(crate) trait ApiResponse: DeserializeOwned + Sized {
    fn from_response(response: reqwest::Response) -> Result<Self, ApiError>;
}
