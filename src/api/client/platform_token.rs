use std::sync::Arc;

use async_std::sync::RwLock;
use jwt_simple::prelude::*;
use time::OffsetDateTime;

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
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformTokenError {
    #[error("failed to generate token for platform platform: {0}")]
    JwtSimpleError(#[from] jwt_simple::Error),
}
