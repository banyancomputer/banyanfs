use serde::Deserialize;

use crate::api::client::{ApiRequest, ApiResponse};

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
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

    fn path(&self) -> String {
        "/api/v1/buckets".to_string()
    }

    fn payload(&self) -> Option<Self::Payload> {
        None
    }
}
