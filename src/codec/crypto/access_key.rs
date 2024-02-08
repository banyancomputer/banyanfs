use chacha20poly1305::{AeadInPlace, Key as ChaChaKey, KeyInit, XChaCha20Poly1305};
use ecdsa::signature::rand_core::CryptoRngCore;
use nom::bytes::streaming::take;
use nom::multi::count;
use nom::sequence::tuple;
use nom::{AsBytes, IResult, Needed};
use rand::{CryptoRng, Rng};

use crate::codec::crypto::{
    AuthenticationTag, KeyId, LockedAccessKey, Nonce, SigningKey, VerifyingKey,
    SYMMETRIC_KEY_LENGTH, TAG_LENGTH,
};

#[derive(Clone)]
pub(crate) struct AccessKey([u8; SYMMETRIC_KEY_LENGTH]);

impl AccessKey {
    pub(crate) fn chacha_key(&self) -> &ChaChaKey {
        ChaChaKey::from_slice(&self.0)
    }

    pub(crate) fn generate(rng: &mut impl CryptoRngCore) -> Self {
        Self(rng.gen())
    }

    pub(crate) fn lock_for(
        &self,
        rng: &mut impl CryptoRngCore,
        verifying_key: &VerifyingKey,
    ) -> Result<LockedAccessKey, AccessKeyError<&[u8]>> {
        let (dh_exchange_key, shared_secret) = verifying_key.ephemeral_dh_exchange(rng);

        // Intentionally leave the last four bytes as zeros which acts as our successful
        // decryption oracle.
        let mut key_payload = [0u8; 36];
        key_payload[..32].copy_from_slice(&self.0);

        let chacha_key = ChaChaKey::from_slice(&shared_secret);
        let cipher = XChaCha20Poly1305::new(chacha_key);

        let nonce = Nonce::generate(rng);
        let raw_tag = cipher.encrypt_in_place_detached(&nonce, &[], &mut key_payload)?;

        let mut tag_bytes = [0u8; TAG_LENGTH];
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
pub(crate) enum AccessKeyError<I> {
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
