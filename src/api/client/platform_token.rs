use std::sync::Arc;
use std::time::Duration;

use async_std::sync::RwLock;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use time::OffsetDateTime;

use crate::api::client::{ExpiringToken, PLATFORM_AUDIENCE};
use crate::codec::crypto::SigningKey;

const CLOCK_LEEWAY: Duration = Duration::from_secs(30);

const TOKEN_LIFETIME: Duration = Duration::from_secs(300);

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

        let current_time = OffsetDateTime::now_utc();

        let not_before = current_time - CLOCK_LEEWAY;
        let expiration = OffsetDateTime::now_utc() + TOKEN_LIFETIME;
        let not_after = expiration + CLOCK_LEEWAY;

        //let token = jwt_key.sign(claims)?;

        let mut rng = crate::utils::crypto_rng();
        let mut nonce_bytes = [0u8; 24];
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = URL_SAFE_NO_PAD.encode(nonce_bytes);

        let jwt_hdr =
            serde_json::json!({"alg": "ES384", "kid": fingerprint, "typ": "JWT"}).to_string();
        let jwt_hdr_b64 = URL_SAFE_NO_PAD.encode(jwt_hdr.as_bytes());

        let jwt_claim = serde_json::json!({
            "iat": current_time.unix_timestamp(),
            "exp": not_after.unix_timestamp(),
            "nbf": not_before.unix_timestamp(),
            "sub": id,
            "aud": PLATFORM_AUDIENCE,
            "nonce": nonce,
        })
        .to_string();
        let jwt_claim_b64 = URL_SAFE_NO_PAD.encode(jwt_claim.as_bytes());

        let signed_data = format!("{}.{}", jwt_hdr_b64, jwt_claim_b64);
        let signature_bytes = key.sign(&mut rng, signed_data.as_bytes()).to_vec();
        let signature = URL_SAFE_NO_PAD.encode(signature_bytes.as_slice());

        // todo generate signature from private key, base64 encode it
        let token = format!("{}.{}", signed_data, signature);

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
