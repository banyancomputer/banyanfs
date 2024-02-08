mod data_header;
mod format_header;
mod identity_header;
mod public_settings;

pub(crate) use data_header::DataHeader;
pub use format_header::FormatHeader;
pub use identity_header::IdentityHeader;
pub(crate) use public_settings::PublicSettings;

const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";
