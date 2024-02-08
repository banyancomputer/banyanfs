use crate::codec::crypto::{KeyId, VerifyingKey};

const FINGERPRINT_SIZE: usize = 32;

pub struct Fingerprint([u8; FINGERPRINT_SIZE]);

impl Fingerprint {
    pub(crate) fn key_id(&self) -> KeyId {
        let mut key_id = [0u8; 2];
        key_id.copy_from_slice(&self.0[..2]);
        KeyId::from(u16::from_le_bytes(key_id))
    }
}

impl std::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fingerprint_str: String = self
            .0
            .iter()
            .fold(String::new(), |acc, &b| format!("{acc}{:02x}", b));

        write!(f, "{{0x{fingerprint_str}}}")
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

#[cfg(test)]
pub mod tests {
    use super::*;

    const REFERENCE_FINGERPRINT_BYTES: &[u8; 32] = b"UUUUUUUUaaaaaaaaUUUUUUUUaaaaaaaa";

    #[test]
    fn test_fingerprint_debug_fmt() {
        let fingerprint = Fingerprint::from(*REFERENCE_FINGERPRINT_BYTES);
        let fmt_str = format!("{:?}", fingerprint);

        assert_eq!(
            fmt_str,
            "{0x5555555555555555616161616161616155555555555555556161616161616161}"
        );
    }

    #[test]
    fn test_key_id_from_fingerprint() {
        let fingerprint = Fingerprint::from(*REFERENCE_FINGERPRINT_BYTES);
        let key_id = fingerprint.key_id();
        assert_eq!(key_id, KeyId::from(0x5555));
    }
}
