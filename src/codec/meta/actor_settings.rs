use crate::codec::crypto::VerifyingKey;
use crate::codec::header::KeyAccessSettings;
use crate::codec::meta::VectorClock;

#[derive(Debug)]
pub struct ActorSettings {
    verifying_key: VerifyingKey,
    access_settings: KeyAccessSettings,
    vector_clock: VectorClock,
}

impl ActorSettings {
    pub fn actor_settings(&self) -> KeyAccessSettings {
        self.access_settings.clone()
    }

    pub const fn size() -> usize {
        VerifyingKey::size() + KeyAccessSettings::size() + VectorClock::size()
    }

    pub fn new(verifying_key: VerifyingKey, access_settings: KeyAccessSettings) -> Self {
        let vector_clock = VectorClock::init();

        Self {
            verifying_key,
            access_settings,
            vector_clock,
        }
    }

    pub fn vector_clock(&self) -> VectorClock {
        self.vector_clock.clone()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key.clone()
    }
}
