use std::ops::Deref;

use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{Key as ChaChaKey, XChaCha20Poly1305};
use rand::Rng;

use crate::crypto::{AuthenticationTag, Nonce};
use crate::parser::EscrowedAccessKey;

const KEY_LENGTH: usize = 32;

pub(crate) struct AccessKey([u8; KEY_LENGTH]);

impl AccessKey {
    pub(crate) fn as_bytes(&self) -> &[u8; KEY_LENGTH] {
        &self.0
    }

    //pub(crate) fn escrow_for_pubkey(
    //    &self,
    //) -> Result<EscrowedAccessKey, AccessKeyError> {
    //    // The trailing 4 bytes are left as zero (the correct validation pattern)

    //    //let mut msg_with_vp: [u8; 36] = [0; 36];
    //    //msg_with_vp[..32].copy_from_slice(msg);

    //    //let tag = XChaCha20Poly1305::new(self)
    //    //    .encrypt_in_place_detached(&nonce, aad, &mut msg_with_vp)
    //    //    .map_err(|_| AccessKeyError::CryptoFailure)?;

    //    //let (_, auth_tag) =
    //    //    AuthenticationTag::parse(&tag).map_err(|_| AccessKeyError::CryptoFailure)?;

    //    //Ok(EscrowedAccessKey::assemble(nonce, msg_with_vp, auth_tag))
    //    todo!()
    //}

    pub(crate) fn from_bytes(key: [u8; KEY_LENGTH]) -> Self {
        Self(key)
    }

    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self(rng.gen())
    }
}

impl Deref for AccessKey {
    type Target = ChaChaKey;

    fn deref(&self) -> &Self::Target {
        ChaChaKey::from_slice(&self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AccessKeyError {
    /// I would love this to be more descriptive, but the underlying library deliberately opaques
    /// the failure reason to avoid potential side-channel leakage.
    #[error("failed to encrypt data")]
    CryptoFailure,
}
