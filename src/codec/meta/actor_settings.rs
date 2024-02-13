use crate::codec::crypto::VerifyingKey;
use crate::codec::header::KeyAccessSettings;

#[derive(Debug)]
pub struct ActorSettings {
    verifying_key: VerifyingKey,
    access_settings: KeyAccessSettings,
}

impl ActorSettings {
    pub fn actor_settings(&self) -> KeyAccessSettings {
        self.access_settings.clone()
    }

    pub fn new(verifying_key: VerifyingKey, access_settings: KeyAccessSettings) -> Self {
        Self {
            verifying_key,
            access_settings,
        }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key.clone()
    }
}
