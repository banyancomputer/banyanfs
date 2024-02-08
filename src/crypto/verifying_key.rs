use p384::NistP384;

use crate::parser::crypto::KeyId;

const KEY_SIZE: usize = 49;

pub(crate) struct VerifyingKey {
    inner_key: ecdsa::VerifyingKey<NistP384>,
}

impl VerifyingKey {
    pub(crate) fn key_id(&self) -> KeyId {
        let public_key_bytes = self.inner_key.to_encoded_point(true);
        let public_key_hash = blake3::hash(public_key_bytes.as_bytes());

        let mut key_id = [0u8; 2];
        key_id.copy_from_slice(public_key_hash.as_bytes());

        KeyId::from(u16::from_le_bytes(key_id))
    }

    pub(crate) fn to_bytes(&self) -> [u8; KEY_SIZE] {
        let compressed_pubkey = self.inner_key.to_encoded_point(true);

        let mut public_key = [0u8; KEY_SIZE];
        public_key.copy_from_slice(compressed_pubkey.as_bytes());

        public_key
    }
}

impl From<ecdsa::VerifyingKey<NistP384>> for VerifyingKey {
    fn from(inner_key: ecdsa::VerifyingKey<NistP384>) -> Self {
        Self { inner_key }
    }
}
