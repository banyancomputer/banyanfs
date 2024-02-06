use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::XChaCha20Poly1305;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::crypto::{AccessKey, AuthenticationTag, CryptoError, Nonce};

pub(crate) fn cs_rng() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}

pub(crate) fn symmetric_decrypt(
    key: &AccessKey,
    nonce: &Nonce,
    cipher_text: &[u8],
    tag: &AuthenticationTag,
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let payload = Payload {
        msg: cipher_text,
        aad,
    };

    let result = XChaCha20Poly1305::new(key)
        .decrypt(nonce, payload)
        .map_err(|_| CryptoError::DecryptionFailure)?;

    Ok(result.to_vec())
}
