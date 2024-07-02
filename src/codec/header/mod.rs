mod access_mask;
mod content_options;
mod identity_header;
mod key_count;
mod public_settings;

pub use access_mask::{AccessMask, AccessMaskBuilder, AccessMaskError};
pub use content_options::ContentOptions;
pub(crate) use identity_header::IdentityHeader;
pub(crate) use key_count::KeyCount;
pub use public_settings::PublicSettings;

pub const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

pub const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";

#[allow(dead_code)]
pub const BANYAN_JOURNAL_MAGIC: &[u8] = b"BYFJ";
