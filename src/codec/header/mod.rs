mod content_options;
mod data_header;
mod encrypted_content_payload_entry;
mod format_header;
mod identity_header;
mod key_access_settings;
mod key_count;
mod permission_control;
mod public_settings;

pub use content_options::ContentOptions;
pub use data_header::DataHeader;
pub use encrypted_content_payload_entry::EncryptedContentPayloadEntry;
pub use format_header::FormatHeader;
pub use identity_header::IdentityHeader;
pub use key_access_settings::{KeyAccessSettings, KeyAccessSettingsBuilder};
pub use key_count::KeyCount;
pub use permission_control::PermissionControl;
pub use public_settings::PublicSettings;

const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";
