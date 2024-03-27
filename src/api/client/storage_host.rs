use std::sync::Arc;

use jwt_simple::prelude::*;
use time::OffsetDateTime;

use crate::api::client::{ExpiringToken, StorageTokenError, STORAGE_HOST_AUDIENCE};
use crate::codec::crypto::SigningKey;

impl StorageHost {
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
        //let paired_id = format!("{account_id}@{}", fingerprint);
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
        jwt_key = jwt_key.with_key_id(&fingerprint);
        let token = jwt_key.sign(claims)?;

        tracing::debug!("generated new storage host token");
        self.token = Some(ExpiringToken::new(token.clone(), expiration));

        Ok(token)
    }

    pub(crate) fn record_storage_grant(&mut self, grant_token: &str) {
        self.pending_storage_grant = Some(grant_token.to_string());
        self.storage_exceeded = false;
    }

    pub(crate) fn storage_exceeded(&mut self) {
        self.storage_exceeded = true;
    }
}
