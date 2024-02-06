use p384::NistP384;

const KEY_SIZE: usize = 49;

pub(crate) struct VerifyingKey {
    inner_key: ecdsa::VerifyingKey<NistP384>,
}

impl VerifyingKey {
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
