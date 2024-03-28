use std::sync::Arc;

use async_std::sync::RwLock;
use reqwest::Url;

use crate::api::client::{
    ApiClient, PlatformToken, PlatformTokenError, StorageHostAuth, StorageTokenError,
};
use crate::codec::crypto::SigningKey;

#[derive(Clone)]
pub(crate) struct ApiAuth {
    account_id: String,
    key: Arc<SigningKey>,
    platform_token: PlatformToken,
    storage_hosts: Arc<RwLock<StorageHostAuth>>,
}

impl ApiAuth {
    pub(crate) async fn platform_token(&self) -> Result<String, PlatformTokenError> {
        self.platform_token
            .get_token(&self.account_id, &self.key)
            .await
    }

    pub(crate) async fn record_storage_grant(&self, storage_host_url: Url, auth_token: &str) {
        let mut storage_hosts = self.storage_hosts.write().await;
        storage_hosts.record_grant(storage_host_url, auth_token.to_string());
    }

    pub fn new(account_id: impl Into<String>, key: Arc<SigningKey>) -> Self {
        let account_id = account_id.into();
        let platform_token = PlatformToken::default();
        let storage_hosts = Arc::new(RwLock::new(StorageHostAuth::default()));

        Self {
            account_id,
            key,

            platform_token,
            storage_hosts,
        }
    }

    pub(crate) fn signing_key(&self) -> Arc<SigningKey> {
        self.key.clone()
    }

    pub(crate) async fn storage_host_token(
        &self,
        client: &ApiClient,
        host_url: &Url,
    ) -> Result<String, StorageTokenError> {
        let mut storage_hosts = self.storage_hosts.write().await;

        storage_hosts
            .get_token(client, host_url, &self.account_id, &self.key)
            .await
    }

    pub(crate) async fn clear_storage_host_auth(&self, host_url: &Url) {
        self.storage_hosts
            .write()
            .await
            .clear_authentication(host_url);
    }
}
