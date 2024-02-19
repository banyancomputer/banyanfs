use async_trait::async_trait;
use reqwest::Method;
use serde::Serialize;

use crate::api::client::ApiRequest;
use crate::api::platform::{ApiDrive, DriveKind, StorageClass};

#[derive(Serialize)]
pub(crate) struct CreateRequest {
    pub(crate) name: String,

    #[serde(rename = "type")]
    pub(crate) kind: DriveKind,

    pub(crate) storage_class: StorageClass,

    #[serde(rename = "initial_bucket_key_pem")]
    pub(crate) owner_key: String,
}

#[async_trait]
impl ApiRequest for CreateRequest {
    type Response = ApiDrive;

    fn method(&self) -> Method {
        Method::POST
    }

    fn path(&self) -> String {
        "/api/v1/buckets".to_string()
    }

    fn is_payload(&self) -> bool {
        true
    }
}