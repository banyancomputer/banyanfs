use crate::codec::crypto::VerifyingKey;
use crate::codec::header::KeyAccessSettings;

#[derive(Debug)]
pub struct ActorSettings {
    verifying_key: VerifyingKey,
    access_settings: KeyAccessSettings,
}
