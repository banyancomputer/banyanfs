use std::collections::HashMap;

use crate::codec::crypto::VerifyingKey;
use crate::codec::ActorId;
use crate::filesystem::KeyAccessSettings;

#[derive(Debug)]
pub struct DriveAccess {
    actor_settings: HashMap<ActorId, (VerifyingKey, KeyAccessSettings)>,
}

impl DriveAccess {
    pub fn access_settings(&self, actor_id: ActorId) -> Option<KeyAccessSettings> {
        self.actor_settings.get(&actor_id).map(|(_, kas)| *kas)
    }

    pub fn has_write_access(&self, actor_id: ActorId) -> bool {
        let settings = match self.actor_settings.get(&actor_id) {
            Some(s) => s,
            None => return false,
        };

        if settings.is_historical() {
            return false;
        }

        match settings {
            KeyAccessSettings::Public { owner, .. } => *owner,
            KeyAccessSettings::Private {
                realized_key_present,
                data_key_present,
                journal_key_present,
                ..
            } => *realized_key_present && *data_key_present && *journal_key_present,
        }
    }
}
