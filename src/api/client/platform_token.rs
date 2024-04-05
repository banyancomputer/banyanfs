use std::sync::Arc;

use async_std::sync::RwLock;

use crate::api::client::utils::create_jwt;
use crate::api::client::{ExpiringToken, PLATFORM_AUDIENCE};
use crate::codec::crypto::SigningKey;

#[derive(Clone, Default)]
pub(crate) struct PlatformToken(Arc<RwLock<Option<ExpiringToken>>>);

impl PlatformToken {
    pub(crate) async fn get_token(
        &self,
        id: &str,
        key: &Arc<SigningKey>,
    ) -> Result<String, PlatformTokenError> {
        // If we already have token and it's not expired, return it
        if let Some(expiring_token) = &*self.0.read().await {
            if let Some(token) = expiring_token.value() {
                return Ok(token);
            }
        }

        let mut rng = crate::utils::crypto_rng();
        let (token, expiration) = create_jwt(&mut rng, id, PLATFORM_AUDIENCE, key);

        tracing::debug!("generated new platform token");

        let stored_token = ExpiringToken::new(token.clone(), expiration);
        let platform_token = &mut *self.0.write().await;
        *platform_token = Some(stored_token);

        tracing::debug!("recorded token");

        Ok(token)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("platform token error")]
pub struct PlatformTokenError;
