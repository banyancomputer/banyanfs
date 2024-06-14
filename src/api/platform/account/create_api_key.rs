use async_trait::async_trait;

use reqwest::{Method, RequestBuilder};
use serde::Serialize;

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiUserKey;

// todo(sstelfox): currently we support registering any key with permissions to the account, then
// that key gets full access to the account and any encrypted data even if the user doesn't have
// access. We need to adjust our permission model to restrict the key usage to the buckets it has
// permission over. We'll need a workflow to upgrade keys to the root of the account for things
// like managing billing.
#[derive(Serialize)]
pub struct CreateApiKey {
    name: String,
    public_key: String,
}

impl CreateApiKey {
    pub fn new(name: &str, public_key: &str) -> Self {
        Self {
            name: name.to_string(),
            public_key: public_key.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl ApiRequest for CreateApiKey {
    type Response = ApiUserKey;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(self))
    }

    fn path(&self) -> String {
        "/api/v1/auth/api_key".to_string()
    }
}

impl PlatformApiRequest for CreateApiKey {}