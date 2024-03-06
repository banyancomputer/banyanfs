use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiDrive;

#[derive(Debug, Serialize)]
pub(crate) struct GetAllRequest;

#[async_trait]
impl ApiRequest for GetAllRequest {
    type Response = Vec<ApiDrive>;

    fn path(&self) -> String {
        "/api/v1/buckets".to_string()
    }
}

impl PlatformApiRequest for GetAllRequest {}
