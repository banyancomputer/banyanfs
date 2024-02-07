mod data;
mod identity;

pub(crate) use data::DataHeader;
pub(crate) use identity::IdentityHeader;

use rand::RngCore;

const BANYAN_FS_MAGIC: &[u8] = b"BYFS";

const BANYAN_DATA_MAGIC: &[u8] = b"BYFD";

pub(crate) struct FilesystemId([u8; 16]);

impl FilesystemId {
    pub(crate) fn generate(rng: &mut impl RngCore) -> Self {
        let mut id = [0; 16];
        id[0] = 0x01;
        Self(id)
    }
}
