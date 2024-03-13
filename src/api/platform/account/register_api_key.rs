use async_trait::async_trait;
use reqwest::Method;
use serde::Serialize;

use crate::api::client::{ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiDriveKey;
use crate::codec::crypto::VerifyingKey;

// todo(sstelfox): currently we support registering any key with permissions to the account, then
// that key gets full access to the account and any encrypted data even if the user doesn't have
// access. We need to adjust our permission model to restrict the key usage to the buckets it has
// permission over. We'll need a workflow to upgrade keys to the root of the account for things
// like managing billing.
#[derive(Serialize)]
pub struct RegisterApiKey {
    #[serde(skip)]
    fingerprint: String,

    public_key: String,
}

impl RegisterApiKey {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn new(public_key: &VerifyingKey) -> Self {
        let fingerprint = public_key.fingerprint().to_hex();
        let public_key = public_key.to_spki().expect("valid key to be encodable");

        Self {
            fingerprint,
            public_key,
        }
    }
}

#[async_trait]
impl ApiRequest for RegisterApiKey {
    type Response = ApiDriveKey;

    const METHOD: Method = Method::POST;

    fn add_payload(&self, request_builder: RequestBuilder) -> RequestBuilder {
        request_builder.json(self)
    }

    fn path(&self) -> String {
        "/api/v1/auth/device_api_key".to_string()
    }
}

impl PlatformApiRequest for RegisterApiKey {}
