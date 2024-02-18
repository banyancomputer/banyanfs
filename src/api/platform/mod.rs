use serde::Deserialize;

use crate::api::client::{ApiRequest, ApiResponse};

#[derive(Debug, Deserialize)]
pub struct ApiDrive {
    id: String,
    name: String,

    #[serde(rename = "type")]
    kind: String,

    storage_class: String,
}

pub struct GetAllDrivesRequest;

impl ApiRequest for GetAllDrivesRequest {
    type Response = Vec<ApiDrive>;

    type Payload = ();

    fn path(&self) -> &str {
        "/api/v1/buckets"
    }

    fn payload(&self) -> Option<Self::Payload> {
        None
    }
}
