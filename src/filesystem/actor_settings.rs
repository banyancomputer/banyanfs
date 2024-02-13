use crate::codec::crypto::VerifyingKey;
use crate::filesystem::KeyAccessSettings;

#[derive(Debug)]
pub struct ActorSettings {
    verifying_key: VerifyingKey,
    access_settings: KeyAccessSettings,
}
