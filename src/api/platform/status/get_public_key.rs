use async_trait::async_trait;
use serde::Deserialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};

pub(crate) struct GetPublicKey;

#[async_trait(?Send)]
impl ApiRequest for GetPublicKey {
    type Response = GetPublicKeyResponse;

    fn path(&self) -> String {
        "/_status/public_key".to_string()
    }
}

impl PlatformApiRequest for GetPublicKey {}

#[derive(Deserialize)]
pub(crate) struct GetPublicKeyResponse {
    #[serde(rename = "pem")]
    public_key: String,
}

impl GetPublicKeyResponse {
    pub fn public_key(&self) -> &str {
        &self.public_key
    }
}
