mod api_auth;
mod direct_response;
mod error;
mod expiring_token;
mod storage_host;
mod storage_host_auth;
mod traits;

pub use error::ApiClientError;

pub(crate) mod utils;

pub(crate) use api_auth::ApiAuth;
pub(crate) use direct_response::DirectResponse;
pub(crate) use expiring_token::ExpiringToken;
pub(crate) use storage_host::StorageHost;
pub(crate) use storage_host_auth::{StorageHostAuth, StorageTokenError};
pub(crate) use traits::{
    ApiRequest, FromReqwestResponse, PlatformApiRequest, StorageHostApiRequest,
};

use std::sync::Arc;

use async_std::sync::RwLock;
use jwt_simple::prelude::*;
use reqwest::{Client, Url};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::debug;

use crate::codec::crypto::SigningKey;
use crate::prelude::BanyanFsError;

pub(crate) const PLATFORM_AUDIENCE: &str = "banyan-platform";

pub(crate) const STORAGE_HOST_AUDIENCE: &str = "banyan-storage";

#[derive(Clone)]
pub struct ApiClient {
    auth: Option<ApiAuth>,
    base_url: Url,
    client: Client,
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

    pub(crate) async fn request<R: ApiRequest>(
        &self,
        base_url: &Url,
        bearer_token: Option<String>,
        mut request: R,
    ) -> Result<Option<R::Response>, ApiError> {
        debug!(method = %R::METHOD, %base_url, url = %request.path(), "request");

        let full_url = base_url.join(&request.path())?;
        let mut request_builder = self.client.request(R::METHOD, full_url);

        if R::REQUIRES_AUTH && bearer_token.is_none() {
            return Err(ApiError::RequiresAuth);
        }

        if let Some(tok) = bearer_token {
            request_builder = request_builder.bearer_auth(tok);
        }

        request_builder = request.add_payload(request_builder).await?;

        let response = request_builder.send().await?;
        let status = response.status();

        debug!(response_status = ?status, "platform_request_response");

        if status.is_success() {
            FromReqwestResponse::from_response(response).await
        } else {
            let resp_bytes = response.bytes().await?;

            match serde_json::from_slice::<RawApiError>(&resp_bytes) {
                Ok(raw_error) => Err(ApiError::Message {
                    status_code: status.as_u16(),
                    message: raw_error.message,
                }),
                Err(_) => {
                    tracing::warn!(response_status = ?status, "api endpoint did not return standard error message");

                    Err(ApiError::Message {
                        status_code: status.as_u16(),
                        message: String::from_utf8_lossy(&resp_bytes).to_string(),
                    })
                }
            }
        }
    }

    pub(crate) async fn platform_request<R: PlatformApiRequest>(
        &self,
        request: R,
    ) -> Result<Option<R::Response>, ApiError> {
        // Send authentication if its available even if the request is not marked as requiring it
        let token = match &self.auth {
            Some(auth) => Some(auth.platform_token().await?),
            None => None,
        };

        self.request(&self.base_url, token, request).await
    }

    pub(crate) async fn platform_request_empty_response<R>(
        &self,
        request: R,
    ) -> Result<(), ApiError>
    where
        R: PlatformApiRequest<Response = ()>,
    {
        let resp = self.platform_request(request).await?;

        if cfg!(feature = "strict") && resp.is_some() {
            return Err(ApiError::UnexpectedResponse("expected empty response"));
        }

        Ok(())
    }

    pub(crate) async fn platform_request_full<R: PlatformApiRequest>(
        &self,
        request: R,
    ) -> Result<R::Response, ApiError> {
        match self.platform_request(request).await? {
            Some(resp) => Ok(resp),
            None => Err(ApiError::UnexpectedResponse("response should not be empty")),
        }
    }

    pub(crate) async fn record_storage_grant(&self, storage_host_url: &Url, auth_token: &str) {
        tracing::info!(?storage_host_url, "registered storage grant");

        let auth = match &self.auth {
            Some(auth) => auth,
            None => {
                tracing::warn!(
                    "registering storage grants without authentication doesn't have any effect"
                );
                return;
            }
        };

        auth.record_storage_grant(storage_host_url, auth_token)
            .await;
    }

    pub(crate) async fn active_storage_host(&self) -> Option<Url> {
        match &self.auth {
            Some(auth) => auth.active_storage_host().await,
            None => None,
        }
    }

    pub(crate) async fn set_active_storage_host(&self, storage_host_url: Url) {
        let auth = match &self.auth {
            Some(auth) => auth,
            None => {
                tracing::warn!(
                    "setting active storage host without authentication doesn't have any effect"
                );
                return;
            }
        };

        auth.set_active_storage_host(storage_host_url).await;
    }

    pub(crate) fn signing_key(&self) -> Option<Arc<SigningKey>> {
        self.auth.as_ref().map(|a| a.signing_key())
    }

    pub(crate) async fn storage_host_request<R: StorageHostApiRequest>(
        &self,
        storage_host_url: &Url,
        request: R,
    ) -> Result<Option<R::Response>, ApiError> {
        // Send authentication if its available even if the request is not marked as requiring it
        let token = match &self.auth {
            Some(auth) => {
                if let Some(grant) = auth.get_storage_grant(storage_host_url).await {
                    crate::api::storage_host::auth::register_grant(&self, storage_host_url, &grant)
                        .await?;
                    auth.clear_storage_grant(storage_host_url).await;
                }

                // todo(sstelfox): there is another side to the auth sequence I haven't done yet,
                // if the storage host doesn't know about this client we need to request a fresh
                // storage grant from the platform before continuing. This whole process is
                // probably worth extracting from this location...

                Some(auth.storage_host_token(storage_host_url).await?)
            }
            None => None,
        };

        self.request(storage_host_url, token, request).await
    }

    pub(crate) async fn storage_host_request_full<R: StorageHostApiRequest>(
        &self,
        storage_host_url: &Url,
        request: R,
    ) -> Result<R::Response, ApiError> {
        match self.storage_host_request(storage_host_url, request).await? {
            Some(resp) => Ok(resp),
            None => Err(ApiError::UnexpectedResponse("response should not be empty")),
        }
    }
}

fn default_reqwest_client() -> Result<reqwest::Client, ApiClientError> {
    let user_agent = format!("banyanfs/{}", crate::version::minimal_version());
    let client = reqwest::Client::builder().user_agent(user_agent).build()?;

    Ok(client)
}

#[derive(Clone, Default)]
pub(crate) struct PlatformToken(Arc<RwLock<Option<ExpiringToken>>>);

impl PlatformToken {
    pub(crate) async fn get_token(
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

        let verifying_key = key.verifying_key();
        let fingerprint = crate::api::client::utils::api_fingerprint_key(&verifying_key);
        let expiration = OffsetDateTime::now_utc() + std::time::Duration::from_secs(300);

        // todo(sstelfox): this jwt library is definitely an integration pain point, we have all
        // the primives already in this crate, we should just use them and correctly construct the
        // JWTs ourselves.
        let current_ts = Clock::now_since_epoch();
        let mut claims = Claims::create(Duration::from_secs(330))
            .with_audience(PLATFORM_AUDIENCE)
            .with_subject(id)
            .invalid_before(current_ts - Duration::from_secs(30));

        claims.create_nonce();
        claims.issued_at = Some(current_ts);

        let mut jwt_key = ES384KeyPair::from_bytes(&key.to_bytes())?;
        jwt_key = jwt_key.with_key_id(&fingerprint);
        let token = jwt_key.sign(claims)?;

        tracing::debug!("generated new platform token");

        let stored_token = ExpiringToken::new(token.clone(), expiration);
        let platform_token = &mut *self.0.write().await;
        *platform_token = Some(stored_token);

        tracing::debug!("recorded token");

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

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("network client experienced issue: {0}")]
    ClientError(#[from] reqwest::Error),

    #[error("the data provided to the API client wasn't valid: {0}")]
    InvalidData(String),

    #[error("Request URL is invalid: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("API returned {status_code} response with message: {message}")]
    Message { status_code: u16, message: String },

    #[error("response from API did not match our expectations: {0}")]
    MismatchedData(String),

    #[error("failed to generate token for platform platform: {0}")]
    PlatformTokenError(#[from] PlatformTokenError),

    #[error("unable to reuse streaming requests")]
    RequestReused,

    #[error("API request requires authentication but client is not authenticated")]
    RequiresAuth,

    #[error("failed to during json (de)serialization: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("failed to generate token for storage host: {0}")]
    StorageTokenError(#[from] StorageTokenError),

    #[error("unexpected I/O error in API client: {0}")]
    StreamingIo(#[from] std::io::Error),

    #[error("unexpected API response: {0}")]
    UnexpectedResponse(&'static str),

    #[cfg(target_arch = "wasm32")]
    #[error("WASM internal error: {0}")]
    WasmInternal(String),
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
