use chacha20poly1305::{AeadInPlace, Key as ChaChaKey, KeyInit, XChaCha20Poly1305};
use ecdsa::signature::rand_core::CryptoRngCore;
use nom::AsBytes;
use rand::Rng;

use crate::codec::crypto::{
    AsymLockedAccessKey, AuthenticationTag, Nonce, SymLockedAccessKey, VerifyingKey,
};

const ACCESS_KEY_LENGTH: usize = 32;

#[derive(Clone)]
pub struct AccessKey([u8; ACCESS_KEY_LENGTH]);

impl AccessKey {
    pub(crate) fn chacha_key(&self) -> &ChaChaKey {
        ChaChaKey::from_slice(&self.0)
    }

    pub fn decrypt_buffer(
        &self,
        nonce: Nonce,
        buffer: &mut [u8],
        tag: AuthenticationTag,
    ) -> Result<(), AccessKeyError<&[u8]>> {
        XChaCha20Poly1305::new(self.chacha_key()).decrypt_in_place_detached(
            &nonce,
            &[],
            buffer,
            &tag,
        )?;

        Ok(())
    }

    pub fn encrypt_buffer(
        &self,
        rng: &mut impl CryptoRngCore,
        buffer: &mut [u8],
    ) -> Result<(Nonce, AuthenticationTag), AccessKeyError<&[u8]>> {
        let cipher = XChaCha20Poly1305::new(self.chacha_key());

        let nonce = Nonce::generate(rng);
        let raw_tag = cipher.encrypt_in_place_detached(&nonce, &[], buffer)?;

        let mut tag_bytes = [0u8; AuthenticationTag::size()];
        tag_bytes.copy_from_slice(raw_tag.as_bytes());
        let tag = AuthenticationTag::from(tag_bytes);

        Ok((nonce, tag))
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        Self(rng.gen())
    }

    pub fn lock_for(
        &self,
        rng: &mut impl CryptoRngCore,
        verifying_key: &VerifyingKey,
    ) -> Result<AsymLockedAccessKey, AccessKeyError<&[u8]>> {
        let (dh_exchange_key, shared_secret) = verifying_key.ephemeral_dh_exchange(rng);

        let mut payload = self.0;
        let (nonce, tag) = shared_secret
            .encrypt_buffer(rng, &mut payload)
            .map_err(|_| AccessKeyError::CryptoFailure)?;

        let key_id = verifying_key.key_id();

        Ok(AsymLockedAccessKey {
            dh_exchange_key,
            nonce,
            cipher_text: payload,
            tag,
            key_id,
        })
    }

    pub fn lock_with(
        &self,
        rng: &mut impl CryptoRngCore,
        encryption_key: &AccessKey,
    ) -> Result<SymLockedAccessKey, AccessKeyError<&[u8]>> {
        let mut payload = self.0;

        let (nonce, tag) = encryption_key
            .encrypt_buffer(rng, &mut payload)
            .map_err(|_| AccessKeyError::CryptoFailure)?;

        Ok(SymLockedAccessKey {
            nonce,
            cipher_text: payload,
            tag,
        })
    }

    pub const fn size() -> usize {
        ACCESS_KEY_LENGTH
    }
}

impl std::fmt::Debug for AccessKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccessKey(*redacted*)")
    }
}

impl From<[u8; ACCESS_KEY_LENGTH]> for AccessKey {
    fn from(key: [u8; ACCESS_KEY_LENGTH]) -> Self {
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
