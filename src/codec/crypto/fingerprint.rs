use crate::codec::crypto::{KeyId, VerifyingKey};

const FINGERPRINT_SIZE: usize = 32;

pub(crate) struct Fingerprint([u8; FINGERPRINT_SIZE]);

impl Fingerprint {
    pub(crate) fn key_id(&self) -> KeyId {
        let mut key_id = [0u8; 2];
        key_id.copy_from_slice(&self.0[..2]);
        KeyId::from(u16::from_le_bytes(key_id))
    }
}

impl std::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fingerprint_str: String = self.0.iter().map(|b| format!("{:02x}", b)).collect();
        write!(f, "{{{fingerprint_str}}}")
    }
}

impl From<[u8; FINGERPRINT_SIZE]> for Fingerprint {
    fn from(bytes: [u8; FINGERPRINT_SIZE]) -> Self {
        Self(bytes)
    }
}

impl From<&VerifyingKey> for Fingerprint {
    fn from(key: &VerifyingKey) -> Self {
        let public_key_bytes = key.to_encoded_point(true);
        Self(blake3::hash(public_key_bytes.as_bytes()).into())
    }
}
