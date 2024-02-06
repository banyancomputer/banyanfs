mod data;
mod identity;

pub(crate) use data::DataHeader;
pub(crate) use identity::IdentityHeader;

const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";
