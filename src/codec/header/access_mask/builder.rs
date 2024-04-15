use crate::codec::header::access_mask::{
    AccessMask, DATA_KEY_PRESENT_BIT, FILESYSTEM_KEY_PRESENT_BIT, HISTORICAL_BIT,
    MAINTENANCE_KEY_PRESENT_BIT, OWNER_BIT, PROTECTED_BIT,
};

pub struct AccessMaskBuilder {
    bits: u8,
}

impl AccessMaskBuilder {
    pub fn build(self) -> AccessMask {
        AccessMask::from(self.bits)
    }

    pub fn full_access() -> Self {
        let mut bits = 0;

        bits |= FILESYSTEM_KEY_PRESENT_BIT;
        bits |= DATA_KEY_PRESENT_BIT;
        bits |= MAINTENANCE_KEY_PRESENT_BIT;

        Self { bits }
    }

    pub fn maintenance() -> Self {
        let mut bits = 0;

        bits |= MAINTENANCE_KEY_PRESENT_BIT;

        Self { bits }
    }

    pub fn set_historical(mut self) -> Self {
        self.bits |= HISTORICAL_BIT;
        self
    }

    pub fn set_owner(mut self) -> Self {
        self.bits |= OWNER_BIT;
        self
    }

    pub fn set_protected(mut self) -> Self {
        self.bits |= PROTECTED_BIT;
        self
    }

    pub fn structural() -> Self {
        let mut bits = 0;

        bits |= FILESYSTEM_KEY_PRESENT_BIT;
        bits |= MAINTENANCE_KEY_PRESENT_BIT;

        Self { bits }
    }
}
