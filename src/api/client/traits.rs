#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use async_trait::async_trait;
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::api::client::ApiError;

// todo(sstelfox): The request itself shouldn't require the Serialize implmentation, I should
// support building a custom payload (and maybe an entire reqwest::Body for stream support). This
// is a lazy hack for speed right now
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
