pub mod data_header;
pub mod format_header;
pub mod identity_header;
pub mod public_settings;

pub use data_header::DataHeader;
pub use format_header::FormatHeader;
pub use identity_header::IdentityHeader;
pub use public_settings::PublicSettings;

const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";
