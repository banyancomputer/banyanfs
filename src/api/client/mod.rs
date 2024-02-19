#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod auth;
mod error;
mod traits;
pub(crate) mod utils;

pub use error::ApiClientError;
pub(crate) use traits::ApiRequest;

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use async_std::sync::RwLock;
use jwt_simple::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client as RClient, Method, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::{debug, Level};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::codec::crypto::SigningKey;
use crate::prelude::BanyanFsError;

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

    pub fn base_url(&self) -> Url {
        self.base_url.clone()
    }

    pub(crate) async fn platform_request<R: ApiRequest>(
        &self,
        request: R,
    ) -> Result<Option<R::Response>, ApiError> {
        debug!(method = %request.method(), url = %request.path(), "platform_request");

        if request.requires_auth() && self.auth.is_none() {
            return Err(ApiError::RequiresAuth);
        }

        let full_url = self.base_url.join(&request.path())?;
        let mut request_builder = self.client.request(request.method(), full_url);

        if let Some(auth) = &self.auth {
            let token = auth.platform_token().await?;
            request_builder = request_builder.bearer_auth(token);
        }

        let request_builder = match request.payload() {
            Some(payload) => request_builder.json(&payload),
            None => request_builder,
        };

        let response = request_builder.send().await?;
        let status = response.status();

        debug!(response_status = ?status, "platform_request_response");

        if status.is_success() {
            if response.status() == StatusCode::NO_CONTENT {
                return Ok(None);
            }

            match response.json::<R::Response>().await {
                Ok(resp) => Ok(Some(resp)),
                Err(err) => {
                    tracing::error!("failed to parse response API: {}", err);

                    Err(ApiError::Message {
                        status_code: status.as_u16(),
                        message: format!("failed to parse response: {err}"),
                    })
                }
            }
        } else {
            let raw_error = response.json::<RawApiError>().await?;

            Err(ApiError::Message {
                status_code: status.as_u16(),
                message: raw_error.message,
            })
        }
    }

    pub(crate) async fn platform_request_with_response<R: ApiRequest>(
        &self,
        request: R,
    ) -> Result<R::Response, ApiError> {
        match self.platform_request(request).await? {
            Some(resp) => Ok(resp),
            None => Err(ApiError::UnexpectedResponse("response should not be empty")),
        }
    }
}

fn default_reqwest_client() -> Result<reqwest::Client, ApiClientError> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let user_agent = format!("banyanfs/{}", crate::version::minimal_version());

    let client = reqwest::Client::builder()
        .default_headers(default_headers)
        .user_agent(user_agent)
        .build()?;

    Ok(client)
}

#[derive(Clone)]
pub(crate) struct ApiAuth {
    account_id: String,
    key: Arc<SigningKey>,

    platform_token: PlatformToken,
    storage_tokens: StorageTokens,
}

impl ApiAuth {
    async fn platform_token(&self) -> Result<String, PlatformTokenError> {
        self.platform_token.get(&self.account_id, &self.key).await
    }

    pub fn new(account_id: impl Into<String>, key: Arc<SigningKey>) -> Self {
        let account_id = account_id.into();
        let platform_token = PlatformToken::default();
        let storage_tokens = StorageTokens::default();

        Self {
            account_id,
            key,

            platform_token,
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
pub(crate) struct PlatformToken(Arc<RwLock<Option<ExpiringToken>>>);

impl PlatformToken {
    pub(crate) async fn get(
        &self,
        id: &str,
        key: &Arc<SigningKey>,
    ) -> Result<String, PlatformTokenError> {
        // If we already have token and it's not expired, return it
        if let Some(token) = &*self.0.read().await {
            if !token.is_expired() {
                return Ok(token.value());
            }
        }

        tracing::debug!("generating new platform token");

        let verifying_key = key.verifying_key();
        let fingerprint = crate::api::client::utils::api_fingerprint_key(&verifying_key);
        let expiration = OffsetDateTime::now_utc() + std::time::Duration::from_secs(300);

        // todo(sstelfox): this jwt library is definitely an integration pain point, we have all
        // the primives already in this crate, we should just use them and correctly construct the
        // JWTs ourselves.
        let current_ts = Clock::now_since_epoch();
        let mut claims = Claims::create(Duration::from_secs(330))
            .with_audiences(HashSet::from_strings(&[PLATFORM_AUDIENCE]))
            .with_subject(id)
            .invalid_before(current_ts - Duration::from_secs(30));

        claims.create_nonce();
        claims.issued_at = Some(current_ts);

        let mut jwt_key = ES384KeyPair::from_bytes(&key.to_bytes())?;
        jwt_key = jwt_key.with_key_id(&fingerprint);
        let token = jwt_key.sign(claims)?;

        let mut writable = self.0.write().await;
        *writable = Some(ExpiringToken::new(token.clone(), expiration));

        Ok(token)
    }

    pub(crate) fn new() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformTokenError {
    #[error("failed to generate token for platform platform: {0}")]
    JwtSimpleError(#[from] jwt_simple::Error),
}

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
    #[error("network client experienced issue: {0}")]
    ClientError(#[from] reqwest::Error),

    #[error("the data provided the API client wasn't valid: {0}")]
    InvalidData(String),

    #[error("Request URL is invalid: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("API returned {status_code} response with message: {message}")]
    Message { status_code: u16, message: String },

    #[error("failed to generate token for platform platform: {0}")]
    PlatformTokenError(#[from] PlatformTokenError),

    #[error("API request requires authentication but client is not authenticated")]
    RequiresAuth,

    #[error("unexpected API response: {0}")]
    UnexpectedResponse(&'static str),
}

impl From<ApiError> for BanyanFsError {
    fn from(error: ApiError) -> Self {
        Self::from(error.to_string())
    }
}

#[derive(Debug, Deserialize)]
pub struct RawApiError {
    #[serde(rename = "msg")]
    pub message: String,
}
