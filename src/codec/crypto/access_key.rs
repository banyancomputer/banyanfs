use chacha20poly1305::{AeadInPlace, Key as ChaChaKey, KeyInit, XChaCha20Poly1305};
use ecdsa::signature::rand_core::CryptoRngCore;
use rand::Rng;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::codec::crypto::{
    AsymLockedAccessKey, AuthenticationTag, Nonce, SymLockedAccessKey, VerifyingKey,
};

const ACCESS_KEY_LENGTH: usize = 32;

#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct AccessKey([u8; ACCESS_KEY_LENGTH]);

impl AccessKey {
    pub(crate) fn chacha_key(&self) -> &ChaChaKey {
        ChaChaKey::from_slice(&self.0)
    }

    pub fn decrypt_buffer(
        &self,
        nonce: Nonce,
        authenticated_data: &[u8],
        buffer: &mut [u8],
        tag: AuthenticationTag,
    ) -> Result<(), AccessKeyError<&[u8]>> {
        let cipher = XChaCha20Poly1305::new(self.chacha_key());
        cipher.decrypt_in_place_detached(&nonce, authenticated_data, buffer, &tag)?;
        Ok(())
    }

    pub fn encrypt_buffer(
        &self,
        rng: &mut impl CryptoRngCore,
        authenticated_data: &[u8],
        buffer: &mut [u8],
    ) -> Result<(Nonce, AuthenticationTag), AccessKeyError<&[u8]>> {
        let cipher = XChaCha20Poly1305::new(self.chacha_key());

        let nonce = Nonce::generate(rng);
        let raw_tag = cipher.encrypt_in_place_detached(&nonce, authenticated_data, buffer)?;

        let mut tag_bytes = [0u8; AuthenticationTag::size()];
        tag_bytes.copy_from_slice(raw_tag.as_slice());
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
            .encrypt_buffer(rng, &[], &mut payload)
            .map_err(|_| AccessKeyError::CryptoFailure)?;

        Ok(AsymLockedAccessKey {
            dh_exchange_key,
            nonce,
            cipher_text: payload,
            tag,
        })
    }

    pub fn lock_with(
        &self,
        rng: &mut impl CryptoRngCore,
        encryption_key: &AccessKey,
    ) -> Result<SymLockedAccessKey, AccessKeyError<&[u8]>> {
        let mut payload = self.0;

        let (nonce, tag) = encryption_key
            .encrypt_buffer(rng, &[], &mut payload)
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
    FormatFailure(#[from] winnow::error::ErrMode<winnow::error::Error<I>>),

    #[error("unspecified crypto error")]
    CryptoFailure,
}

impl<I> From<chacha20poly1305::Error> for AccessKeyError<I> {
    fn from(_: chacha20poly1305::Error) -> Self {
        AccessKeyError::CryptoFailure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::codec::crypto::SigningKey;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_encryption_buffer_roundtrip() {
        let mut rng = crate::utils::crypto_rng();

        let reference_pt = b"nothing hidden".to_vec();
        let mut buffer = reference_pt.clone();
        let access_key = AccessKey::generate(&mut rng);

        let (nonce, tag) = access_key
            .encrypt_buffer(&mut rng, &[], &mut buffer)
            .expect("encryption success");

        assert_ne!(&reference_pt, &buffer);

        access_key
            .decrypt_buffer(nonce, &[], &mut buffer, tag)
            .expect("decryption success");

        assert_eq!(&reference_pt, &buffer);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_locking_leaves_original_unchanged() {
        const REFERENCE_KEY: [u8; ACCESS_KEY_LENGTH] = [0x55; ACCESS_KEY_LENGTH];

        let access_key = AccessKey::from(REFERENCE_KEY);

        let mut rng = crate::utils::crypto_rng();
        let locking_sym_key = AccessKey::generate(&mut rng);
        access_key
            .lock_with(&mut rng, &locking_sym_key)
            .expect("sym encryption");

        assert_eq!(access_key.0, REFERENCE_KEY);

        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();

        access_key
            .lock_for(&mut rng, &verifying_key)
            .expect("asym encryption");

        assert_eq!(access_key.0, REFERENCE_KEY);
    }
}
