use async_trait::async_trait;
use serde::Serialize;

use crate::api::client::ApiRequest;
use crate::api::platform::ApiDrive;

#[derive(Debug, Serialize)]
pub struct GetAllDrivesRequest;

#[async_trait]
impl ApiRequest for GetAllDrivesRequest {
    type Response = Vec<ApiDrive>;

    fn path(&self) -> String {
        "/api/v1/buckets".to_string()
    }
}
