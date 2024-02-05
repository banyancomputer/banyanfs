pub(crate) struct EncryptedPayload(Vec<u8>);

impl EncryptedPayload {
    pub(crate) fn to_bytes(&self) -> &[u8] {
        &self.0
    }
}
