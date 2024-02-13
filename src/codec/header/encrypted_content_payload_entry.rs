use crate::codec::crypto::{AccessKey, AuthenticationTag, KeyId, Nonce, VerifyingKey};
use crate::codec::header::{KeyCount, PermissionControl};

const ENCRYPTED_KEY_PAYLOAD_SIZE: usize = KeyId::size()
    + VerifyingKey::size()
    + Nonce::size()
    + AccessKey::size()
    + AuthenticationTag::size();

pub struct EncryptedContentPayloadEntry;

impl EncryptedContentPayloadEntry {
    pub fn parse(
        input: &[u8],
        key_count: KeyCount,
        access_key: AccessKey,
    ) -> nom::IResult<&[u8], Vec<PermissionControl>> {
        todo!()
    }
}
