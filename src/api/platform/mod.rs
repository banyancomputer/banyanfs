use serde::{Deserialize, Serialize};

use crate::api::client::ApiRequest;

#[derive(Debug, Deserialize, Serialize)]
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
