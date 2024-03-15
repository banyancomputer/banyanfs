use async_trait::async_trait;
use reqwest::Method;

use crate::api::client::{ApiRequest, PlatformApiRequest};

pub(crate) struct DeleteRequest {
    drive_id: String,
}

impl DeleteRequest {
    pub(crate) fn new(drive_id: String) -> Self {
        Self { drive_id }
    }
}

#[async_trait]
impl ApiRequest for DeleteRequest {
    type Response = ();

    const METHOD: Method = Method::DELETE;

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}", self.drive_id)
    }
}

impl PlatformApiRequest for DeleteRequest {}
