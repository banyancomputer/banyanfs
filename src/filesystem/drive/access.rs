use std::collections::HashMap;

use crate::codec::crypto::VerifyingKey;
use crate::codec::header::KeyAccessSettings;
use crate::codec::{ActorId, ActorSettings};

#[derive(Debug, Default)]
pub struct DriveAccess {
    actor_settings: HashMap<ActorId, ActorSettings>,
}

impl DriveAccess {
    pub fn actor_settings(&self, actor_id: ActorId) -> Option<KeyAccessSettings> {
        self.actor_settings
            .get(&actor_id)
            .map(|settings| settings.actor_settings())
    }

    pub fn has_realized_view_access(&self, actor_id: ActorId) -> bool {
        let settings = match self.actor_settings.get(&actor_id) {
            Some(s) => s.actor_settings(),
            None => return false,
        };

        if settings.is_historical() {
            return false;
        }

        match settings {
            KeyAccessSettings::Public { .. } => true,
            KeyAccessSettings::Private {
                realized_key_present,
                ..
            } => realized_key_present,
        }
    }

    pub fn has_write_access(&self, actor_id: ActorId) -> bool {
        let settings = match self.actor_settings.get(&actor_id) {
            Some(s) => s.actor_settings(),
            None => return false,
        };

        if settings.is_historical() {
            return false;
        }

        match settings {
            KeyAccessSettings::Public { owner, .. } => owner,
            KeyAccessSettings::Private {
                realized_key_present,
                data_key_present,
                journal_key_present,
                ..
            } => realized_key_present && data_key_present && journal_key_present,
        }
    }

    pub fn new() -> Self {
        Self {
            actor_settings: HashMap::new(),
        }
    }

    pub fn register_actor(&mut self, key: VerifyingKey, settings: KeyAccessSettings) {
        let actor_id = key.actor_id();
        let actor_settings = ActorSettings::new(key, settings);

        self.actor_settings.insert(actor_id, actor_settings);
    }
}
