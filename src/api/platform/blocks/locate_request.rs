use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::Url;
use reqwest::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::codec::Cid;

#[derive(Serialize)]
pub(crate) struct LocateRequest {
    cids: Vec<String>,
}

impl LocateRequest {
    pub(crate) fn new(cids: Vec<Cid>) -> Self {
        let cids = cids
            .into_iter()
            .map(|c| c.as_base64url_multicodec())
            .collect::<Vec<_>>();

        Self { cids }
    }
}

#[async_trait(?Send)]
impl ApiRequest for LocateRequest {
    type Response = InnerLocateResponse;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(&self))
    }

    fn path(&self) -> String {
        "/api/v1/blocks/locate".to_string()
    }
}

impl PlatformApiRequest for LocateRequest {}

// note(sstelfox): one of the keys is "NA" indicating the blocks couldn't be found, this map is Url
// -> Vec<Cid>.
#[derive(Deserialize)]
pub(crate) struct InnerLocateResponse(HashMap<String, Vec<String>>);

pub(crate) struct LocateResponse {
    storage_hosts: Vec<Url>,
    cid_location: HashMap<Cid, Vec<usize>>,
}

impl From<InnerLocateResponse> for LocateResponse {
    fn from(value: InnerLocateResponse) -> Self {
        todo!()
    }
}
