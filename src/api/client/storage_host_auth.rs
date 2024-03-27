use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use reqwest::Url;

use crate::api::client::ExpiringToken;
use crate::codec::crypto::SigningKey;

#[derive(Default)]
pub(crate) struct StorageHostAuth {
    active_tokens: HashMap<Url, ExpiringToken>,
    authenticated_storage_hosts: HashSet<Url>,
    _pending_grants: HashMap<Url, String>,
}

impl StorageHostAuth {
    pub(crate) fn get_token(
        &mut self,
        _storage_host_url: &Url,
        _account_id: &str,
        _key: &Arc<SigningKey>,
    ) -> Result<String, StorageTokenError> {
        // Check if we have any pending grants for the storage host
        // - If so attempt to register it with the storage host and clear it locally
        // - If it succeeds mark the host as authenticated

        // If this storage host is listed as authenticated
        // - Check if we have an active token, if so return it
        // - If not generate a new one, register it in the cache, and return it

        // Perform a who_am_i request against it
        // - On success add it to the authenticated storage hosts set, generate,
        //   cache, and return a token
        // - On failure or the available capacity is too low, request an updated
        //   grant from the platform and register it with the storage host

        // Last resort is to _assume_ we're authenticated, generate, and cache a token anyway. A
        // not authorized error will be handled by the client by clearing the host from the list of
        // authenticated hosts.
        todo!()
    }

    // todo(sstelfox): need to call from the storage host client requests
    pub(crate) fn not_authenticated(&mut self, storage_host_url: &Url) {
        self.active_tokens.remove(storage_host_url);
        self.authenticated_storage_hosts.remove(storage_host_url);
    }

    pub(crate) async fn record_storage_grant(&mut self, _storage_host_url: Url, _auth_token: &str) {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StorageTokenError {
    #[error("failed to generate token for a storage host: {0}")]
    JwtSimpleError(#[from] jwt_simple::Error),
}
