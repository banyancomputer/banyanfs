#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod auth;
mod error;
mod traits;

pub use error::ApiClientError;
pub(crate) use traits::{ApiRequest, ApiResponse};

use std::collections::BTreeMap;
use std::sync::Arc;

use async_std::sync::RwLock;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client as RClient, Method, Response, Url};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::codec::crypto::SigningKey;

pub(crate) const PLATFORM_AUDIENCE: &str = "banyan-platform";

pub struct ApiClient {
    auth: Option<ApiAuth>,
    base_url: Url,
    client: RClient,
}

impl ApiClient {
    pub fn anonymous(base_url: &str) -> Result<Self, ApiClientError> {
        let client = default_reqwest_client()?;
        let base_url = Url::parse(base_url)?;

        Ok(Self {
            auth: None,
            base_url,
            client,
        })
    }

    pub fn authenticated(
        base_url: &str,
        account_id: &str,
        key: Arc<SigningKey>,
    ) -> Result<Self, ApiClientError> {
        let base_url = Url::parse(base_url)?;
        let auth = Some(ApiAuth::new(account_id, key));
        let client = default_reqwest_client()?;

        Ok(Self {
            auth,
            base_url,
            client,
        })
    }

    pub(crate) async fn send_request<R: ApiRequest>(
        &self,
        request: &R,
    ) -> Result<Option<R::Response>, ApiError> {
        let full_url = self.base_url.join(request.path())?;
        let req = self.client.request(request.method(), full_url);

        if request.requires_auth() && self.auth.is_none() {
            return Err(ApiError::RequiresAuth);
        }

        todo!()
    }
}

fn default_reqwest_client() -> Result<RClient, ApiClientError> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let user_agent = format!("banyanfs/{}", crate::version::minimal_version());

    let client = RClient::builder()
        .default_headers(default_headers)
        .user_agent(user_agent)
        .build()?;

    Ok(client)
}

#[derive(Clone)]
pub(crate) struct ApiAuth {
    account_id: String,
    key: Arc<SigningKey>,

    core_token: CoreToken,
    storage_tokens: StorageTokens,
}

impl ApiAuth {
    async fn core_token(&self) -> Result<String, CoreTokenError> {
        self.core_token.get(&self.account_id, &self.key).await
    }

    pub fn new(account_id: impl Into<String>, key: Arc<SigningKey>) -> Self {
        let account_id = account_id.into();
        let core_token = CoreToken::default();
        let storage_tokens = StorageTokens::default();

        Self {
            account_id,
            key,

            core_token,
            storage_tokens,
        }
    }

    async fn storage_token(&self, host: &str) -> Result<String, StorageTokenError> {
        self.storage_tokens
            .get(host, &self.account_id, &self.key)
            .await
    }
}

#[derive(Clone, Default)]
pub(crate) struct CoreToken(Arc<RwLock<Option<ExpiringToken>>>);

impl CoreToken {
    pub(crate) async fn get(
        &self,
        _id: &str,
        _key: &Arc<SigningKey>,
    ) -> Result<String, CoreTokenError> {
        // If we already have token and it's not expired, return it
        if let Some(token) = &*self.0.read().await {
            if !token.is_expired() {
                return Ok(token.value());
            }
        }

        // todo: generate a proper JWT here
        let new_expiration = OffsetDateTime::now_utc() + time::Duration::minutes(5);
        let new_token = String::new();

        let mut writable = self.0.write().await;
        *writable = Some(ExpiringToken::new(new_token.clone(), new_expiration));

        Ok(new_token)
    }

    pub(crate) fn new() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CoreTokenError {}

#[derive(Clone, Default)]
pub(crate) struct StorageTokens {
    known_tokens: Arc<RwLock<BTreeMap<String, ExpiringToken>>>,
}

impl StorageTokens {
    pub(crate) async fn get(
        &self,
        _host: &str,
        _id: &str,
        _key: &Arc<SigningKey>,
    ) -> Result<String, StorageTokenError> {
        todo!()
    }

    pub(crate) fn new() -> Self {
        let known_tokens = Arc::new(RwLock::new(BTreeMap::new()));
        Self { known_tokens }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StorageTokenError {}

#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct ExpiringToken {
    token: String,

    #[zeroize(skip)]
    expiration: OffsetDateTime,
}

impl ExpiringToken {
    pub(crate) fn is_expired(&self) -> bool {
        self.expiration < OffsetDateTime::now_utc()
    }

    pub(crate) fn new(token: String, expiration: OffsetDateTime) -> Self {
        Self { token, expiration }
    }

    pub(crate) fn value(&self) -> String {
        self.token.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("API returned {status_code} response with message: {message}")]
    Message { status_code: u16, message: String },

    #[error("API request requires authentication but client is not authenticated")]
    RequiresAuth,

    #[error("Request URL is invalid: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

#[derive(Debug, Deserialize)]
pub struct RawApiError {
    #[serde(rename = "msg")]
    pub message: String,
}
