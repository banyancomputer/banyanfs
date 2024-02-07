use rand::{Rng, RngCore};

const ID_LENGTH: usize = 16;

pub(crate) struct FilesystemId([u8; ID_LENGTH]);

impl FilesystemId {
    pub(crate) fn generate(rng: &mut impl RngCore) -> Self {
        let id: [u8; ID_LENGTH] = rng.gen();
        Self(id)
    }
}
