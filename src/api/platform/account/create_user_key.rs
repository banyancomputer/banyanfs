use async_trait::async_trait;
use elliptic_curve::pkcs8::{EncodePublicKey, LineEnding};
use reqwest::{Method, RequestBuilder};
use serde::Serialize;

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiUserKey;
use crate::codec::crypto::VerifyingKey;

// todo(sstelfox): currently we support registering any key with permissions to the account, then
// that key gets full access to the account and any encrypted data even if the user doesn't have
// access. We need to adjust our permission model to restrict the key usage to the buckets it has
// permission over. We'll need a workflow to upgrade keys to the root of the account for things
// like managing billing.
#[derive(Serialize)]
pub struct CreateUserKey {
    name: String,
    public_key_pem: String,
}

impl CreateUserKey {
    pub fn new(name: &str, public_key_pem: &str) -> Self {
        Self {
            name: name.to_string(),
            public_key_pem: public_key_pem.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl ApiRequest for CreateUserKey {
    type Response = ApiUserKey;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(self))
    }

    fn path(&self) -> String {
        "/api/v1/auth/user_key".to_string()
    }
}

impl PlatformApiRequest for CreateUserKey {}
