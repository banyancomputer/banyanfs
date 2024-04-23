use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use reqwest::Url;

use crate::api::client::utils::create_jwt;
use crate::api::client::{ApiClient, ApiError, ExpiringToken, STORAGE_HOST_AUDIENCE};
use crate::api::platform::account::get_storage_grant;
use crate::api::storage_host::auth::{register_grant, who_am_i};
use crate::codec::crypto::SigningKey;

#[derive(Default)]
pub struct StorageHostAuth {
    active_tokens: HashMap<Url, ExpiringToken>,
    authenticated_storage_hosts: HashSet<Url>,
    pending_grants: HashMap<Url, String>,
}

impl StorageHostAuth {
    /// Reset any cached authentication details stored for the provided Url. Expects the base URL
    /// of the specific storage host. The URL is both case and scheme specific.
    pub fn clear_authentication(&mut self, storage_host_url: &Url) {
        self.active_tokens.remove(storage_host_url);
        self.authenticated_storage_hosts.remove(storage_host_url);
    }

    /// Attempts to retrieve a current Bearer token if one is available. If there is a token an its
    /// expired or the storage host is unknown, this will return None.
    fn current_token(&self, storage_host_url: &Url) -> Option<String> {
        self.active_tokens
            .get(storage_host_url)
            .and_then(ExpiringToken::value)
    }

    fn generate_token(
        &mut self,
        storage_host_url: &Url,
        account_id: &str,
        key: &Arc<SigningKey>,
    ) -> Result<String, StorageTokenError> {
        let mut rng = crate::utils::crypto_rng();
        let (token, expiration) = create_jwt(&mut rng, account_id, STORAGE_HOST_AUDIENCE, key);

        self.active_tokens.insert(
            storage_host_url.clone(),
            ExpiringToken::new(token.clone(), expiration),
        );

        tracing::debug!(%storage_host_url, "generated new storage host token");

        Ok(token)
    }

    pub(crate) async fn get_token(
        &mut self,
        client: &ApiClient,
        storage_host_url: &Url,
        account_id: &str,
        key: &Arc<SigningKey>,
    ) -> Result<String, StorageTokenError> {
        // Check if we have any pending grants for the storage host
        // - If so attempt to register it with the storage host and clear it locally
        // - If it succeeds mark the host as authenticated
        // - Continue on failure
        if let Some(grant) = self.pending_grants.remove(storage_host_url) {
            if let Err(err) = register_grant(client, storage_host_url, &grant).await {
                tracing::warn!(
                    "failed to register pending grant with storage host: {}",
                    err
                );
            }
        }

        // If this storage host is listed as authenticated
        // - Check if we have an active token, if so return it
        // - If not generate a new one, register it in the cache, and return it
        if self.authenticated_storage_hosts.contains(storage_host_url) {
            if let Some(t) = self.current_token(storage_host_url) {
                return Ok(t);
            }

            // Since we know we're authenticated with the host we just create a new token and use
            // that. If we loose our authentication or get a not authorized we're removed from this
            // list and will do the extended authentication.
            return self.generate_token(storage_host_url, account_id, key);
        }

        // We're not explicitly aware that we're authenticated, but our key might have interacted
        // with the storage host in the past, we'll try and generate token and check if the storage
        // host already knows us. If we have to register with the storage host, this token will
        // become valid afterwards so we mind as well cache it.
        let new_token = match self.current_token(storage_host_url) {
            Some(t) => t,
            None => self.generate_token(storage_host_url, account_id, key)?,
        };

        // Perform a who_am_i request against it
        // - On success add it to the authenticated storage hosts set, generate, cache, and return a token
        // - On not authorized, request an updated grant from the platform and register it with
        //   the storage host
        match who_am_i(client, storage_host_url, &new_token).await {
            Ok(_) => {
                self.authenticated_storage_hosts
                    .insert(storage_host_url.clone());
                return Ok(new_token);
            }
            Err(ApiError::NotAuthorized) => {
                // We're not currently authorized we need to get a grant from the platform and
                // attempt to register it.
                match get_storage_grant(client, storage_host_url.clone()).await {
                    Ok(grant) => {
                        self.register_grant(
                            client,
                            storage_host_url.clone(),
                            grant.authorization_token(),
                        )
                        .await;
                    }
                    Err(err) => {
                        tracing::error!("failed to retrieve storage grant from platform: {}", err);
                        return Err(StorageTokenError::PlatformGrant);
                    }
                }
            }
            Err(err) => {
                tracing::error!(
                    "unexpected error attempted to check authentication status of storage host: {}",
                    err
                );
                return Err(StorageTokenError::StorageHostApi);
            }
        }

        // We're pretty sure at thit point that we've either succeeded or explicitly failed against
        // the storage host, we'll return the token we generated earlier to attempt the request
        // that is about to happen, but its unlikely to succeed.
        tracing::warn!("falling back to storage host token that may not be authorized for access");
        Ok(new_token)
    }

    pub fn record_grant(&mut self, storage_host_url: Url, grant: String) {
        self.pending_grants.insert(storage_host_url, grant);
    }

    /// Attempts to register a storage grant with a storage host. If this registration fails, we
    /// log the result but in all cases we attempt to continue with the authentication process as
    /// the update may not be needed to proceed so failures are not tracked.
    async fn register_grant(&mut self, client: &ApiClient, storage_host_url: Url, grant: &str) {
        match register_grant(client, &storage_host_url, grant).await {
            Ok(_) => {
                self.authenticated_storage_hosts.insert(storage_host_url);
            }
            Err(err) => {
                tracing::error!("failed to register grant with storage host: {}", err);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StorageTokenError {
    #[error("failed to retrieve storage grant from platform")]
    PlatformGrant,

    #[error("failed to check authentication against storage host")]
    StorageHostApi,
}
