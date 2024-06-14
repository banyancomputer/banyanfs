mod vec_stream;

pub use vec_stream::VecStream;

use std::time::Duration;

use async_std::prelude::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use blake3::Hasher;
use bytes::{Bytes, BytesMut};
use elliptic_curve::rand_core::CryptoRngCore;
use time::OffsetDateTime;

use crate::codec::crypto::{SigningKey, VerifyingKey};

const CLOCK_LEEWAY: Duration = Duration::from_secs(30);

const FINGERPRINT_SIZE: usize = 20;

const TOKEN_LIFETIME: Duration = Duration::from_secs(300);

/// The API uses a truncated hex encoded blake3 hash for key identification in its JWTs. This
/// generates the odd version specifically for that generation and should not be used for other
/// things.
///
/// todo(sstelfox): This needs to be reverted back to a standard size (its tech debt)
pub(crate) fn api_fingerprint_key(key: &VerifyingKey) -> String {
    let compressed_point = key.to_encoded_point(true);
    let compressed_point = compressed_point.as_bytes();

    let mut hasher = Hasher::new();
    hasher.update(compressed_point);
    let mut hash_reader = hasher.finalize_xof();

    let mut output = [0u8; FINGERPRINT_SIZE];
    hash_reader.fill(&mut output);

    output.iter().fold(String::new(), |mut acc, byte| {
        acc.push_str(&format!("{:02x}", byte));
        acc
    })
}

/// Consumes an async stream into a single Bytes object. This will consume potentially boundless
/// memory which is especially problematic since we will be handling very large files. It is
/// intended primarily for WASM targeted builds where async is significantly more limited.
pub(crate) async fn consume_stream_into_bytes<S, E>(mut stream: S) -> Result<Bytes, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::error::Error,
{
    let mut bytes_mut = BytesMut::new();

    while let Some(item) = stream.next().await {
        let bytes = item?;
        bytes_mut.extend_from_slice(&bytes);
    }

    Ok(bytes_mut.freeze())
}

/// Creates a JWT token to authenticated against the APIs. There are crates that perform this but
/// they are more general and have a much larger attack surface (as well as dependencies with known
/// vulnerabilities). This is a minimal implementation that generates exactly what we need.
pub(crate) fn create_jwt(
    rng: &mut impl CryptoRngCore,
    subject: &str,
    audience: &str,
    key: &SigningKey,
) -> (String, OffsetDateTime) {
    let verifying_key = key.verifying_key();
    let fingerprint = crate::api::client::utils::api_fingerprint_key(&verifying_key);

    let current_time = OffsetDateTime::now_utc();

    let not_before = current_time - CLOCK_LEEWAY;
    let expiration = OffsetDateTime::now_utc() + TOKEN_LIFETIME;
    let not_after = expiration + CLOCK_LEEWAY;

    let mut nonce_bytes = [0u8; 24];
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = URL_SAFE_NO_PAD.encode(nonce_bytes);

    let jwt_hdr = serde_json::json!({"alg": "ES384", "kid": fingerprint, "typ": "JWT"}).to_string();
    let jwt_hdr_b64 = URL_SAFE_NO_PAD.encode(jwt_hdr.as_bytes());

    let jwt_claim = serde_json::json!({
        "iat": current_time.unix_timestamp(),
        "exp": not_after.unix_timestamp(),
        "nbf": not_before.unix_timestamp(),
        "sub": subject,
        "aud": audience,
        "nonce": nonce,
    })
    .to_string();
    let jwt_claim_b64 = URL_SAFE_NO_PAD.encode(jwt_claim.as_bytes());

    let signed_data = format!("{}.{}", jwt_hdr_b64, jwt_claim_b64);
    let signature_bytes = key.sign(rng, signed_data.as_bytes()).to_vec();
    let signature = URL_SAFE_NO_PAD.encode(signature_bytes.as_slice());

    let token = format!("{}.{}", signed_data, signature);

    (token, expiration)
}
