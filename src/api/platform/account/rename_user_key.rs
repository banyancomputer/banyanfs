use async_trait::async_trait;
use elliptic_curve::pkcs8::{EncodePublicKey, LineEnding};
use reqwest::{Method, RequestBuilder};
use serde::Serialize;

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::api::platform::ApiUserKey;
use crate::codec::crypto::VerifyingKey;

#[derive(Serialize)]
pub struct RenameUserKey {
    name: String,
    #[serde(skip)]
    user_key_id: String,
}

impl RenameUserKey {
    pub fn new(name: &str, user_key_id: &str) -> Self {
        Self {
            name: name.to_string(),
            user_key_id: user_key_id.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl ApiRequest for RenameUserKey {
    type Response = ApiUserKey;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(self))
    }

    fn path(&self) -> String {
        format!("/api/v1/auth/user_key/{}", self.user_key_id)
    }
}

impl PlatformApiRequest for RenameUserKey {}
