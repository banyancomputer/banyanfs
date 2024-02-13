mod data_header;
mod format_header;
mod identity_header;
mod key_count;
mod public_settings;

pub use data_header::DataHeader;
pub use format_header::FormatHeader;
pub use identity_header::IdentityHeader;
pub use key_count::KeyCount;
pub use public_settings::PublicSettings;

const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";
