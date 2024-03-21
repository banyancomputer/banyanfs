#[allow(dead_code)]
mod direct_response;
mod error;
mod traits;

pub use error::ApiClientError;

pub(crate) mod utils;

pub(crate) use direct_response::DirectResponse;
pub(crate) use traits::{
    ApiRequest, FromReqwestResponse, PlatformApiRequest, StorageHostApiRequest,
};

use std::collections::HashMap;
use std::sync::Arc;

use async_std::sync::RwLock;
use jwt_simple::prelude::*;
use reqwest::{Client, Url};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::debug;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::api::storage_host;
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

    pub(crate) async fn set_active_storage_host(&self, storage_host_url: &Url) {
        let mut storage_hosts = match &self.auth {
            Some(auth) => auth.storage_hosts.write().await,
            None => {
                tracing::warn!(
                    "setting active storage host without authentication doesn't have any effect"
                );
                return;
            }
        };

        storage_hosts.set_active_storage_host(storage_host_url);
    }

    pub(crate) fn signing_key(&self) -> Option<Arc<SigningKey>> {
        self.auth.as_ref().map(|a| a.key.clone())
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
                    storage_host::auth::register_grant(&self, storage_host_url, &grant).await?;
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

    pub(crate) async fn storage_host_request_empty_response<R>(
        &self,
        storage_host_url: &Url,
        request: R,
    ) -> Result<(), ApiError>
    where
        R: StorageHostApiRequest<Response = ()>,
    {
        let resp = self.storage_host_request(storage_host_url, request).await?;

        if cfg!(feature = "strict") && resp.is_some() {
            return Err(ApiError::UnexpectedResponse("expected empty response"));
        }

        Ok(())
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

#[derive(Default)]
pub(crate) struct StorageHost {
    pending_storage_grant: Option<String>,
    token: Option<ExpiringToken>,
}

impl StorageHost {
    pub(crate) fn clear_storage_grant(&mut self) {
        self.pending_storage_grant.take();
    }

    pub(crate) fn get_storage_grant(&self) -> Option<String> {
        self.pending_storage_grant.clone()
    }

    pub(crate) fn get_token(
        &mut self,
        account_id: &str,
        key: &Arc<SigningKey>,
    ) -> Result<String, StorageTokenError> {
        if let Some(token) = &self.token {
            if !token.is_expired() {
                return Ok(token.value());
            }
        }

        let verifying_key = key.verifying_key();
        let fingerprint = crate::api::client::utils::api_fingerprint_key(&verifying_key);
        let token_kid = format!("{account_id}@{}", fingerprint);
        let expiration = OffsetDateTime::now_utc() + std::time::Duration::from_secs(300);

        // todo(sstelfox): this jwt library is definitely an integration pain point, we have all
        // the primives already in this crate, we should just use them and correctly construct the
        // JWTs ourselves.
        let current_ts = Clock::now_since_epoch();
        let mut claims = Claims::create(Duration::from_secs(330))
            .with_audience(STORAGE_HOST_AUDIENCE)
            // note(sstelfox): I don't believe we're using the subject...
            .with_subject(account_id)
            .invalid_before(current_ts - Duration::from_secs(30));

        claims.create_nonce();
        claims.issued_at = Some(current_ts);

        let mut jwt_key = ES384KeyPair::from_bytes(&key.to_bytes())?;
        jwt_key = jwt_key.with_key_id(&token_kid);
        let token = jwt_key.sign(claims)?;

        tracing::debug!("generated new storage host token");
        self.token = Some(ExpiringToken::new(token.clone(), expiration));

        Ok(token)
    }

    pub(crate) fn record_storage_grant(&mut self, grant_token: &str) {
        self.pending_storage_grant = Some(grant_token.to_string())
    }
}

#[derive(Clone)]
pub(crate) struct ApiAuth {
    account_id: String,
    key: Arc<SigningKey>,
    platform_token: PlatformToken,
    storage_hosts: Arc<RwLock<StorageHostsAuth>>,
}

impl ApiAuth {
    async fn platform_token(&self) -> Result<String, PlatformTokenError> {
        self.platform_token
            .get_token(&self.account_id, &self.key)
            .await
    }

    pub(crate) async fn clear_storage_grant(&self, storage_host_url: &Url) {
        let mut storage_hosts = self.storage_hosts.write().await;
        storage_hosts.clear_storage_grant(storage_host_url);
    }

    pub(crate) async fn get_storage_grant(&self, storage_host_url: &Url) -> Option<String> {
        let mut storage_hosts = self.storage_hosts.write().await;
        storage_hosts.get_storage_grant(storage_host_url)
    }

    pub(crate) async fn record_storage_grant(&self, storage_host_url: &Url, auth_token: &str) {
        let mut storage_hosts = self.storage_hosts.write().await;
        storage_hosts
            .record_storage_grant(storage_host_url, auth_token)
            .await;
    }

    pub fn new(account_id: impl Into<String>, key: Arc<SigningKey>) -> Self {
        let account_id = account_id.into();
        let platform_token = PlatformToken::default();
        let storage_hosts = Arc::new(RwLock::new(StorageHostsAuth::default()));

        Self {
            account_id,
            key,

            platform_token,
            storage_hosts,
        }
    }

    async fn active_storage_host(&self) -> Option<Url> {
        let storage_hosts = self.storage_hosts.read().await;
        storage_hosts.active_storage_host()
    }

    async fn storage_host_token(&self, host_url: &Url) -> Result<String, StorageTokenError> {
        let mut storage_hosts = self.storage_hosts.write().await;
        storage_hosts.get_token(host_url, &self.account_id, &self.key)
    }
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

#[derive(Default)]
struct StorageHostsAuth {
    active_storage_host: Option<Url>,
    storage_hosts: HashMap<Url, StorageHost>,
}

impl StorageHostsAuth {
    pub(crate) fn active_storage_host(&self) -> Option<Url> {
        // todo(sstelfox): could fallback by randomly selecting from the known storage hosts
        self.active_storage_host.clone()
    }

    pub(crate) fn clear_storage_grant(&mut self, storage_host_url: &Url) {
        if let Some(shu) = self.storage_hosts.get_mut(storage_host_url) {
            shu.clear_storage_grant();
        }
    }

    pub(crate) fn get_storage_grant(&mut self, storage_host_url: &Url) -> Option<String> {
        let host = self
            .storage_hosts
            .entry(storage_host_url.clone())
            .or_default();

        host.get_storage_grant()
    }

    pub(crate) fn set_active_storage_host(&mut self, storage_host_url: &Url) {
        self.active_storage_host = Some(storage_host_url.clone());
    }

    pub(crate) fn get_token(
        &mut self,
        storage_host_url: &Url,
        account_id: &str,
        key: &Arc<SigningKey>,
    ) -> Result<String, StorageTokenError> {
        let host = self
            .storage_hosts
            .entry(storage_host_url.clone())
            .or_default();

        host.get_token(account_id, key)
    }

    pub(crate) async fn record_storage_grant(&mut self, storage_host_url: &Url, auth_token: &str) {
        let host = self
            .storage_hosts
            .entry(storage_host_url.clone())
            .or_default();

        host.record_storage_grant(auth_token);
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StorageTokenError {
    #[error("failed to generate token for a storage host: {0}")]
    JwtSimpleError(#[from] jwt_simple::Error),
}

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
