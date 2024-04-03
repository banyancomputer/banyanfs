use async_trait::async_trait;
use reqwest::{Method, RequestBuilder};
use serde::Serialize;

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiKey;
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
        let fingerprint = public_key.fingerprint().as_hex();
        let public_key = public_key.to_spki().expect("valid key to be encodable");

        Self {
            fingerprint,
            public_key,
        }
    }
}

#[async_trait(?Send)]
impl ApiRequest for RegisterApiKey {
    type Response = ApiKey;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(self))
    }

    fn path(&self) -> String {
        "/api/v1/auth/device_api_key".to_string()
    }
}

impl PlatformApiRequest for RegisterApiKey {}
