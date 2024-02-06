use std::ops::Deref;

use chacha20poly1305::XNonce as ChaChaNonce;
use rand::Rng;

pub(crate) struct Nonce([u8; 24]);

impl Nonce {
    pub(crate) fn as_bytes(&self) -> &[u8; 24] {
        &self.0
    }

    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self(rng.gen())
    }
}

impl Deref for Nonce {
    type Target = ChaChaNonce;

    fn deref(&self) -> &Self::Target {
        ChaChaNonce::from_slice(&self.0)
    }
}
