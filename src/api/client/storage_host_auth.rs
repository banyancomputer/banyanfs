use std::collections::HashMap;
use std::sync::Arc;

use reqwest::Url;

use crate::api::client::StorageHost;
use crate::codec::crypto::SigningKey;

#[derive(Default)]
pub(crate) struct StorageHostAuth {
    active_storage_host: Option<Url>,
    storage_hosts: HashMap<Url, StorageHost>,
}

impl StorageHostAuth {
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

    pub(crate) fn set_active_storage_host(&mut self, storage_host_url: Url) {
        self.active_storage_host = Some(storage_host_url);
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
pub enum StorageTokenError {
    #[error("failed to generate token for a storage host: {0}")]
    JwtSimpleError(#[from] jwt_simple::Error),
}
