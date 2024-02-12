use ecdsa::signature::rand_core::CryptoRngCore;

use crate::codec::crypto::AccessKey;
use crate::codec::Cid;
use crate::filesystem::KeyMap;

pub struct PrivateEncodingContext {
    pub(crate) registered_keys: KeyMap,

    pub(crate) key_access_key: AccessKey,

    pub(crate) realized_view_key: AccessKey,
    pub(crate) journal_key: AccessKey,
    pub(crate) maintenance_key: AccessKey,
    pub(crate) data_key: AccessKey,

    pub(crate) journal_vector_range: (u64, u64),
    pub(crate) merkle_root_range: (Cid, Cid),
}

impl PrivateEncodingContext {
    pub fn new(
        rng: &mut impl CryptoRngCore,
        registered_keys: KeyMap,
        journal_vector_range: (u64, u64),
        merkle_root_range: (Cid, Cid),
    ) -> Self {
        let key_access_key = AccessKey::generate(rng);

        let realized_view_key = AccessKey::generate(rng);
        let journal_key = AccessKey::generate(rng);
        let maintenance_key = AccessKey::generate(rng);
        let data_key = AccessKey::generate(rng);

        Self {
            registered_keys,
            key_access_key,
            realized_view_key,
            journal_key,
            maintenance_key,
            data_key,
            journal_vector_range,
            merkle_root_range,
        }
    }
}
