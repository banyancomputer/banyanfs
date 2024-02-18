#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::collections::BTreeMap;
use std::sync::Arc;

use async_std::sync::RwLock;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Method, Response, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::codec::crypto::SigningKey;

const PLATFORM_AUDIENCE: &str = "banyan-platform";

pub(crate) trait ApiRequestTrait {
    type Response: ApiResponseTrait + DeserializeOwned;

    fn requires_auth(&self) -> bool;
}

pub(crate) trait ApiResponseTrait: Sized {
    fn from_response(response: Response) -> Result<Self, ApiError>;
}

#[derive(Clone)]
pub struct ApiClient {
    auth: Option<ApiAuth>,
    base_url: Url,
    client: Client,
}

impl ApiClient {
    pub(crate) async fn call<T>(&self, request: &T) -> Result<T::Response, ApiError>
    where
        T: ApiRequestTrait,
    {
        todo!()
    }

    pub fn new(base_url: Url) -> Self {
        Self {
            auth: None,
            base_url,
            client: default_reqwest_client(),
        }
    }

    pub fn with_auth(base_url: Url, account_id: String, key: Arc<SigningKey>) -> Self {
        let auth = Some(ApiAuth::new(account_id, key));

        Self {
            auth,
            base_url,
            client: default_reqwest_client(),
        }
    }
}

fn default_reqwest_client() -> Client {
    let mut default_headers = HeaderMap::new();
    default_headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    todo!()
}

#[derive(Clone)]
pub(crate) struct ApiAuth {
    account_id: String,
    key: Arc<SigningKey>,

    core_token: CoreToken,
    storage_tokens: StorageTokens,
}

impl ApiAuth {
    pub fn new(account_id: String, key: Arc<SigningKey>) -> Self {
        let core_token = CoreToken::default();
        let storage_tokens = StorageTokens::default();

        Self {
            account_id,
            key,

            core_token,
            storage_tokens,
        }
    }

    async fn core_token(&self) -> Result<String, CoreTokenError> {
        self.core_token.get(&self.account_id, &self.key).await
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

#[derive(Debug)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct RawApiError {
    #[serde(rename = "msg")]
    pub message: String,
}
