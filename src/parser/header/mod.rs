mod data;
mod filesystem_id;
mod format;
mod identity;
mod public_settings;

pub(crate) use data::DataHeader;
pub(crate) use filesystem_id::FilesystemId;
pub(crate) use format::FormatHeader;
pub(crate) use identity::IdentityHeader;
pub(crate) use public_settings::PublicSettings;

const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";
