mod data_header;
mod filesystem_id;
mod format_header;
mod identity_header;
mod public_settings;

pub(crate) use data_header::DataHeader;
pub(crate) use filesystem_id::FilesystemId;
pub(crate) use format_header::FormatHeader;
pub(crate) use identity_header::IdentityHeader;
pub(crate) use public_settings::PublicSettings;

const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";
