use async_trait::async_trait;
use reqwest::{Method, RequestBuilder};
use serde::Serialize;

use crate::api::client::{ApiError, ApiRequest, StorageHostApiRequest};
use crate::codec::crypto::VerifyingKey;

pub(crate) struct RegisterGrantRequest {
    grant_token: String,
    public_key: VerifyingKey,
}

impl RegisterGrantRequest {
    pub(crate) fn new(public_key: VerifyingKey, grant_token: String) -> Self {
        Self {
            grant_token,
            public_key,
        }
    }
}

#[async_trait(?Send)]
impl ApiRequest for RegisterGrantRequest {
    type Response = ();

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        let public_key = self.public_key.to_spki().map_err(|err| {
            let err_msg = format!("public key couldn't be encoded: {err}");
            ApiError::InvalidData(err_msg)
        })?;

        let inner = InnerRequest { public_key };

        Ok(request_builder
            .bearer_auth(self.grant_token.clone())
            .json(&inner))
    }

    fn path(&self) -> String {
        "/api/v1/client_grant".to_string()
    }
}

#[derive(Serialize)]
struct InnerRequest {
    public_key: String,
}

impl StorageHostApiRequest for RegisterGrantRequest {}
