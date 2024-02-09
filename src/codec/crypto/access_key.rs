use chacha20poly1305::{AeadInPlace, Key as ChaChaKey, KeyInit, XChaCha20Poly1305};
use ecdsa::signature::rand_core::CryptoRngCore;
use nom::AsBytes;
use rand::Rng;

use crate::codec::crypto::{
    AuthenticationTag, LockedAccessKey, Nonce, VerifyingKey, SYMMETRIC_KEY_LENGTH,
};

#[derive(Clone)]
pub struct AccessKey([u8; SYMMETRIC_KEY_LENGTH]);

impl AccessKey {
    #[allow(dead_code)]
    pub(crate) fn chacha_key(&self) -> &ChaChaKey {
        ChaChaKey::from_slice(&self.0)
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        Self(rng.gen())
    }

    pub fn lock_for(
        &self,
        rng: &mut impl CryptoRngCore,
        verifying_key: &VerifyingKey,
    ) -> Result<LockedAccessKey, AccessKeyError<&[u8]>> {
        let (dh_exchange_key, shared_secret) = verifying_key.ephemeral_dh_exchange(rng);

        let mut key_payload = [0u8; SYMMETRIC_KEY_LENGTH];
        key_payload.copy_from_slice(&self.0);

        let chacha_key = ChaChaKey::from_slice(&shared_secret);
        let cipher = XChaCha20Poly1305::new(chacha_key);

        let nonce = Nonce::generate(rng);
        let raw_tag = cipher.encrypt_in_place_detached(&nonce, &[], &mut key_payload)?;

        let mut tag_bytes = [0u8; AuthenticationTag::size()];
        tag_bytes.copy_from_slice(raw_tag.as_bytes());
        let tag = AuthenticationTag::from(tag_bytes);

        let key_id = verifying_key.key_id();

        Ok(LockedAccessKey {
            dh_exchange_key,
            nonce,
            cipher_text: key_payload,
            tag,
            key_id,
        })
    }
}

impl From<[u8; SYMMETRIC_KEY_LENGTH]> for AccessKey {
    fn from(key: [u8; SYMMETRIC_KEY_LENGTH]) -> Self {
        Self(key)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AccessKeyError<I> {
    #[error("decoding data failed: {0}")]
    FormatFailure(#[from] nom::Err<nom::error::Error<I>>),

    #[error("unspecified crypto error")]
    CryptoFailure,
}

impl<I> From<chacha20poly1305::Error> for AccessKeyError<I> {
    fn from(_: chacha20poly1305::Error) -> Self {
        AccessKeyError::CryptoFailure
    }
}
