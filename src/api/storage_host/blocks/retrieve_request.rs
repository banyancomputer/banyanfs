use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::{ApiRequest, DirectResponse, StorageHostApiRequest};

#[derive(Serialize)]
pub(crate) struct RetrieveRequest {
    block_cid: String,
}

impl RetrieveRequest {
    pub(crate) fn new(block_cid: &str) -> Self {
        let block_cid = block_cid.to_string();
        Self { block_cid }
    }
}

#[async_trait(?Send)]
impl ApiRequest for RetrieveRequest {
    type Response = DirectResponse;

    fn path(&self) -> String {
        format!("/api/v1/blocks/{}", self.block_cid)
    }
}

impl StorageHostApiRequest for RetrieveRequest {}
