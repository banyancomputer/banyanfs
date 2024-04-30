use crate::codec::header::access_mask::{
    AccessMask, AccessMaskError, DATA_KEY_PRESENT_BIT, FILESYSTEM_KEY_PRESENT_BIT, HISTORICAL_BIT,
    MAINTENANCE_KEY_PRESENT_BIT, OWNER_BIT, PROTECTED_BIT,
};

/// A helper for building up an appropriate [`AccessMask`] instance with different combinations of
/// settings. This currently does not protect against some invalid states so should be used
/// carefully.
pub struct AccessMaskBuilder {
    bits: u8,
}

impl AccessMaskBuilder {
    /// Converts the builder into the final [`AccessMask`] instance.
    pub fn build(self) -> Result<AccessMask, AccessMaskError> {
        AccessMask::try_from(self.bits)
    }

    /// Create a new instance that has access to all of the encryption keys. This doesn't grant
    /// ownership or protection which need to added individually if desired.
    pub fn full_access() -> Self {
        let mut bits = 0;

        bits |= FILESYSTEM_KEY_PRESENT_BIT;
        bits |= DATA_KEY_PRESENT_BIT;
        bits |= MAINTENANCE_KEY_PRESENT_BIT;

        Self { bits }
    }

    /// Set the historical bit for the actor. This effectively removes all the effective access
    /// but otherwise preserves the current permissions bits. An actor with this set remains
    /// present so signatures used by that actor can still be validated.
    ///
    /// Marking an actor as historical must not be done on a protected actor.
    pub fn historical(mut self) -> Self {
        self.bits |= HISTORICAL_BIT;
        self
    }

    /// Crate a new instance that only access to maintenance information. This key provides access
    /// to underlying block add/remove operations details. This does not provide access to the
    /// filesystem structure, attributes, or to any data within the filesystem.
    pub fn maintenance() -> Self {
        let mut bits = 0;

        bits |= MAINTENANCE_KEY_PRESENT_BIT;

        Self { bits }
    }

    /// Set the owner bit for the actor. This grants full access to the filesystem and grants the
    /// ability to manage all the access of the filesystem as well.
    pub fn owner(mut self) -> Self {
        self.bits |= OWNER_BIT;
        self
    }

    /// Mark an actor as protected. This prevents any actor including owners from removing or
    /// changing a key without explictly removing the protection. It's invalid to have this flag
    /// set this when the historical bit is set.
    ///
    /// The invalid states are not currently enforced, it is up to the caller to ensure they don't
    /// create an invalid access mask.
    pub fn protected(mut self) -> Self {
        self.bits |= PROTECTED_BIT;
        self
    }

    /// Create a new instance that has access to everything but the data referenced by the
    /// filesystem.
    pub fn structural() -> Self {
        let mut bits = 0;

        bits |= FILESYSTEM_KEY_PRESENT_BIT;
        bits |= MAINTENANCE_KEY_PRESENT_BIT;

        Self { bits }
    }
}
