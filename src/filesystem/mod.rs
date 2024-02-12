mod content_reference;
mod file_content;
mod nodes;

pub use content_reference::ContentReference;
pub use file_content::FileContent;
pub use nodes::*;

use std::collections::HashMap;

use crate::codec::content_payload::KeyAccessSettings;
use crate::codec::crypto::{SigningKey, VerifyingKey};
use crate::codec::{ActorId, FilesystemId};

pub struct Drive {
    _filesystem_id: FilesystemId,
    _keys: HashMap<ActorId, (VerifyingKey, KeyAccessSettings)>,
}

impl Drive {
    pub fn initialize_private(signing_key: &SigningKey) -> Self {
        let mut rng = crate::utils::crypto_rng();

        let verifying_key = signing_key.verifying_key();
        let fingerprint = signing_key.fingerprint();
        let actor_id = ActorId::from(fingerprint);

        let kas = KeyAccessSettings::Private {
            protected: true,
            owner: true,
            historical: false,

            realized_key_present: true,
            data_key_present: true,
            journal_key_present: true,
            maintenance_key_present: true,
        };

        let mut keys = HashMap::new();
        keys.insert(actor_id, (verifying_key, kas));

        Self {
            _filesystem_id: FilesystemId::generate(&mut rng),
            _keys: keys,
        }
    }
}
