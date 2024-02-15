use crate::codec::crypto::{AccessKey, KeyId, VerifyingKey};
use crate::codec::header::KeyAccessSettings;

#[derive(Debug)]
pub struct PermissionControl {
    key_id: KeyId,
    verifying_key: VerifyingKey,
    access_settings: KeyAccessSettings,

    realized_view_key: Option<AccessKey>,
    journal_key: Option<AccessKey>,
    maitenance_key: Option<AccessKey>,
    data_key: Option<AccessKey>,
}