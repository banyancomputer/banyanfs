use chacha20poly1305::aead::{AeadInPlace, KeyInit, Payload};
use chacha20poly1305::XChaCha20Poly1305;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

//use crate::crypto::{AuthenticationTag, CryptoError, Nonce};
use crate::parser::AccessKey;

/// Data exceeding this length should use the streaming primitives instead.
const SHORT_ENCRYPTION_LENGTH_LIMIT: usize = 262_144;

pub(crate) fn cs_rng() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}

//pub(crate) fn short_symmetric_decrypt(
//    key: &AccessKey,
//    nonce: &Nonce,
//    cipher_text: &[u8],
//    tag: &AuthenticationTag,
//    aad: &[u8],
//) -> Result<Vec<u8>, CryptoError> {
//    let mut plain_text = cipher_text.to_vec();
//
//    XChaCha20Poly1305::new(key)
//        .decrypt_in_place_detached(nonce, aad, &mut plain_text, tag)
//        .map_err(|_| CryptoError::DecryptionFailure)?;
//
//    Ok(plain_text)
//}
//
//pub(crate) fn short_symmetric_encrypt(
//    key: &AccessKey,
//    nonce: &Nonce,
//    msg: &[u8],
//    aad: &[u8],
//) -> Result<(Vec<u8>, AuthenticationTag), CryptoError> {
//    let mut cipher_text = msg.to_vec();
//
//    let tag = XChaCha20Poly1305::new(key)
//        .encrypt_in_place_detached(nonce, aad, &mut cipher_text)
//        .map_err(|_| CryptoError::EncryptionFailure)?;
//
//    let auth_tag =
//        AuthenticationTag::parse_complete(&tag).map_err(|_| CryptoError::EncryptionFailure)?;
//
//    Ok((cipher_text, auth_tag))
//}
