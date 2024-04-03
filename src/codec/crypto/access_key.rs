use chacha20poly1305::{AeadInPlace, Key as ChaChaKey, KeyInit, XChaCha20Poly1305};
use ecdsa::signature::rand_core::CryptoRngCore;
use nom::AsBytes;
use rand::Rng;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::codec::crypto::{
    AsymLockedAccessKey, AuthenticationTag, Nonce, SymLockedAccessKey, VerifyingKey,
};

/// The number of bytes that make up an AccessKey. Under the hood we use XChaCha20Poly1305 which
/// uses 256-bit keys.
pub const ACCESS_KEY_LENGTH: usize = 32;

/// An AccessKey is an unencrypted symmetric key used for encrypting and decrypting arbitrary data.
/// This is used on data blocks, protecting header and filesystem settings, as well as other keys.
/// This is heavily opionated for its purpose but should still be used with care.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct AccessKey([u8; ACCESS_KEY_LENGTH]);

impl AccessKey {
    pub(crate) fn chacha_key(&self) -> &ChaChaKey {
        ChaChaKey::from_slice(&self.0)
    }

    /// Decrypts the provided buffer in place using the internal key, and the provided nonce, tag,
    /// and additional authenticated data. Buffer's length should be a multiple of
    /// ACCESS_KEY_LENGTH.
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

    /// Encrypt the provided buffer in place using the internal key and provided authenticated
    /// data. The returned [`Nonce`] and [`AuthenticationTag`] need to be stored alongside the
    /// ciphertext in-order to decrypt it later. In most cases, this will be handled for you by the
    /// library itself. The buffer should be a multiple of ACCESS_KEY_LENGTH.
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
        tag_bytes.copy_from_slice(raw_tag.as_bytes());
        let tag = AuthenticationTag::from(tag_bytes);

        Ok((nonce, tag))
    }

    /// Create a new [`AccessKey`] using the provided random number generator.
    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        Self(rng.gen())
    }

    /// [`AccessKey`] instances are designed to be shared and protected using asymmetric keys from
    /// within this library. We frequently want to share a private key with another entity that we
    /// have the public key of. This method facilitates this by performing an ephemeral
    /// Diffie-Hellman key exchange between and ephemerally generated key pair and the target
    /// public key.
    ///
    /// Thist method returns a struct that contains all the information necessary to complete the
    /// DH key exchange by the intended recipient. Additional details on getting access to the
    /// encrypted key can be found in the [`AsymLockedAccessKey`] struct.
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

    /// Some [`AccessKey`] instances are protected by other symmetric keys, most notably each file
    /// has its own unique encryption key that is protected with the filesystem's general data
    /// encryption key. This produces a different struct than the lock_for that does not involve
    /// additional asymmetric keys. Refer to [`SymLockedAccessKey`] for additional details on
    /// decryption.
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

    /// Reports the size of this data struct when its encoded or decoded. This struct isn't
    /// directly encodable or parsable as the plaintext is never supposed to be exposed but knowing
    /// the size of this is important for any encoding or decoding that will include the encrypted
    /// form of the key (which doesn't change size itself).
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
    /// When decoding the data that is supposed to represent an [`AccessKey`] is not represented as
    /// expected. This error frequently looks like a stream of bytes and its not always clear from
    /// that why the decoding failed. There are future improvements in the work to improve these
    /// kinds of errors by switching off of the `nom` library but for now additional context is
    /// recommended to help identify what the specific failure was.
    #[error("decoding data failed: {0}")]
    FormatFailure(#[from] nom::Err<nom::error::Error<I>>),

    /// The underlying libraries used by this one for its cryptograhic operations do not provide a
    /// more specific error type so we can not provide more detailed errors than the operation
    /// failed. This is intentional to prevent leaking information about the cryptographic
    /// operations and usually the context of the failure is sufficient to identify why the error
    /// occurred.
    ///
    /// It it important for consumers of this error to provide additional context on the nature of
    /// the error when this occurs so that users have a more informed notion of how this failure
    /// came about.
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
