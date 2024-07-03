use std::collections::HashMap;

use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::AsyncWrite;

use crate::codec::{
    crypto::{AccessKey, KeyId, SigningKey, VerifyingKey},
    header::{AccessMask, AccessMaskBuilder, AccessMaskError},
    ActorId, ActorSettings, ActorSettingsError, ParserResult, Stream,
};

/// [`DriveAccess`] maintains a mapping of [`ActorId`] instances to their available permissions
/// within the drive itself. When loaded this holds on to copies of any of the general keys the
/// current actor has access to.
///
/// Access within the drive is broken up based on which internal symmetric keys the current user
/// has access to. These keys consist of a filesystem key (which grants access to read the
/// structure and metadata of the filesystem), a data key which protects the per-file encryption
/// key needed to access the data contained in any blocks stored), and a maintenance key (which is
/// only used to read block lifecycle information found in different metadata version syncs).
#[derive(Clone, Debug)]
pub struct DriveAccess {
    actor_settings: HashMap<ActorId, ActorSettings>,

    filesystem_key: Option<AccessKey>,
    data_key: Option<AccessKey>,
    maintenance_key: Option<AccessKey>,
}

impl DriveAccess {
    /// Similar to [`DriveAccess::actor_access`], this performs an additional check to reject
    /// historical keys limiting queries to the common case of keys that should currently have some
    /// level of access.
    pub fn active_actor_access(&self, actor_id: &ActorId) -> Option<AccessMask> {
        self.actor_access(actor_id)
            .filter(|access| !access.is_historical())
    }

    /// Queries the drive instance for the specific actor's current permissions. If the actor
    /// doesn't have any permissions this will return None.
    pub fn actor_access(&self, actor_id: &ActorId) -> Option<AccessMask> {
        self.actor_settings
            .get(actor_id)
            .map(|settings| settings.access())
    }

    /// Get the full [`VerifyingKey`] for a specific actor. If the actor doesn't have access to the
    /// drive this will return None.
    pub fn actor_key(&self, actor_id: &ActorId) -> Option<VerifyingKey> {
        self.actor_settings
            .get(actor_id)
            .map(|settings| settings.verifying_key())
            .clone()
    }

    /// A user is allowed to fork off a copy of a drive if they have read access to the data and
    /// the filesystem. At the time, this is equivalent of [`DriveAccess::has_data_access`].
    pub fn can_fork(&self, actor_id: &ActorId) -> bool {
        self.has_data_access(actor_id)
    }

    pub(crate) fn data_key(&self) -> Option<&AccessKey> {
        self.data_key.as_ref()
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        _rng: &mut impl CryptoRngCore,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        if self.actor_settings.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "drive must have at least one registered actor",
            ));
        }

        for settings in self.sorted_actor_settings().iter() {
            let verifying_key = settings.verifying_key();
            let key_id = verifying_key.key_id();

            written_bytes += key_id.encode(writer).await?;

            // todo(sstelfox): user agent version was being updated here and that isn't happening
            // anywhere now. We need to move that up to the Drive encoder
            written_bytes += settings.encode(writer).await?;
        }

        Ok(written_bytes)
    }

    pub(crate) fn filesystem_key(&self) -> Option<&AccessKey> {
        self.filesystem_key.as_ref()
    }

    /// Checks whether the requested actor is able to read and write access to the data referenced
    /// inside the filesystem. This requires both access to see the structure of the filesystem
    /// (which will provide access to file specific key) and the data key (which will decrypt the
    /// file specific key, providing the key for the relevant data blocks).
    ///
    /// Historical keys will always return false.
    ///
    /// With access to the complete data block store, this effectively is access to the entirety of
    /// the current version of the filesystem. File and directory specific permissions may cause
    /// other actors to reject changes when they don't appropriate write permissions.
    pub fn has_data_access(&self, actor_id: &ActorId) -> bool {
        let access = match self.active_actor_access(actor_id) {
            Some(a) => a,
            None => return false,
        };

        access.has_filesystem_key() && access.has_data_key()
    }

    /// Cheks whether the requested actor is able to read maintenance related information from an
    /// encrypted drive. This includes information such as which public keys are present and the
    /// CIDs of the data blocks that are being added and removed.
    pub fn has_maintenance_access(&self, actor_id: &ActorId) -> bool {
        let access = match self.active_actor_access(actor_id) {
            Some(a) => a,
            None => return false,
        };

        access.has_maintenance_key()
    }

    /// Checks whether the requested actor is able to read the the structure of the filesystem and
    /// the associated attributes with its contents. This does not check whether the actor is able
    /// to read data from files or associated data nodes. To check whether the Actor can read data
    /// as well you'll want to use [`DriveAccess::has_data_access`].
    ///
    /// Historical keys will always return false.
    pub fn has_read_access(&self, actor_id: &ActorId) -> bool {
        let access = match self.active_actor_access(actor_id) {
            Some(a) => a,
            None => return false,
        };

        access.has_filesystem_key()
    }

    /// Checks whether the current user is allowed to make changes to the filesystem. While many
    /// operations that only change filesystem metadata could be written without data access, we
    /// currently don't handle that edge case and its of limited use (you would be able to rename
    /// or change permissions but not read or write file data).
    ///
    /// This only checks whether the user has access to the cryptographic keys needed to perform
    /// this kind of operation and does not guarantee other particpants operating on the filesystem
    /// will accept the updates from others. Individual file and directories have permissions that
    /// determine which actors are allowed to make more fine grained changes and are implemented
    /// when consuming journal streams (which will reject delta updates from actor's that ignore
    /// file or directory permissions).
    ///
    /// Historical keys will always return false.
    pub fn has_write_access(&self, actor_id: &ActorId) -> bool {
        let access = match self.active_actor_access(actor_id) {
            Some(a) => a,
            None => return false,
        };

        access.has_filesystem_key() && access.has_data_key() && access.has_maintenance_key()
    }

    /// Create a new DriveAccess instance with the provided actor as an owner with full
    /// permissions.
    pub(crate) fn initialize(
        rng: &mut impl CryptoRngCore,
        verifying_key: VerifyingKey,
    ) -> Result<Self, DriveAccessError> {
        let mut access = Self {
            actor_settings: HashMap::new(),

            filesystem_key: Some(AccessKey::generate(rng)),
            data_key: Some(AccessKey::generate(rng)),
            maintenance_key: Some(AccessKey::generate(rng)),
        };

        let access_mask = AccessMaskBuilder::full_access()
            .owner()
            .protected()
            .build()?;

        access.register_actor(rng, verifying_key, access_mask)?;

        Ok(access)
    }

    pub fn is_historical(&self, actor_id: &ActorId) -> bool {
        match self.active_actor_access(actor_id) {
            Some(a) => a.is_historical(),
            None => false,
        }
    }

    /// Checks whether the actor is currently marked as an owner of the drive in the access
    /// settings. Actors with this permission have a bit of in-built protection against certain
    /// kinds of access changes and are allowed to bypass normal access checks on file and
    /// directory permissions.
    ///
    /// The exception to this is any actor that has the historical flag marked on them. These are
    /// actors that used to be an owner but have since had their access duly revoked by an
    /// authorized user.
    pub fn is_owner(&self, actor_id: &ActorId) -> bool {
        match self.active_actor_access(actor_id) {
            Some(a) => a.is_owner(),
            None => false,
        }
    }

    /// Checks whether the actor is currently marked as protected. Changes to protected keys can
    /// only be made by the actor that owns the key or owners. Deletion or marking as historical
    /// needs to have its protected flag removed first even when the actor is an owner.
    ///
    /// It is invalid for a protected key to be marked as historical, but in the event that
    /// internal state is represented, the protected flag will be ignored and this will return
    /// false.
    pub fn is_protected(&self, actor_id: &ActorId) -> bool {
        match self.active_actor_access(actor_id) {
            Some(a) => a.is_protected(),
            None => false,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn maintenance_key(&self) -> Option<&AccessKey> {
        self.maintenance_key.as_ref()
    }

    pub fn parse<'a>(
        input: Stream<'a>,
        key_count: u8,
        signing_key: &SigningKey,
    ) -> ParserResult<'a, Self> {
        if key_count == 0 {
            return Err(winnow::error::ErrMode::Cut(
                winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Verify,
                ),
            ));
        }

        // todo(sstelfox): It would be much easier on encoders/parsers if the key count was present
        // here... It's one extra byte and would allow us to omit historical keys from the outer
        // header. It'd just be a breaking format change

        let mut actor_settings = HashMap::new();
        let mut buf_slice = input;

        for _ in 0..key_count {
            // todo(sstelfox): we don't need this anymore but its a breaking change to remove it
            let (i, _key_id) = KeyId::parse(buf_slice)?;
            buf_slice = i;

            let (i, settings) = ActorSettings::parse(buf_slice)?;
            buf_slice = i;

            let actor_id = settings.verifying_key().actor_id();
            actor_settings.insert(actor_id, settings);
        }

        let mut drive_access = Self {
            actor_settings,

            filesystem_key: None,
            data_key: None,
            maintenance_key: None,
        };

        if let Err(err) = drive_access.unlock_keys(signing_key) {
            tracing::error!("failed to unlock permission keys: {}", err);

            return Err(winnow::error::ErrMode::Cut(
                winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Verify,
                ),
            ));
        }

        Ok((buf_slice, drive_access))
    }

    /// Adds a new actor (via their [`VerifyingKey`]) to the drive with the provided permissions.
    /// This will produce an error if you attempt to add an actor that already has access or if the
    /// current actor doesn't have access to the permissions you're attempting to grant.
    pub fn register_actor(
        &mut self,
        rng: &mut impl CryptoRngCore,
        key: VerifyingKey,
        access_mask: AccessMask,
    ) -> Result<(), DriveAccessError> {
        // todo(sstelfox): need to add a check that prevents a user from granting privileges beyond
        // their own (they couldn't anyways as they don't have access to the symmetric keys
        // necessary, but we should prevent invalid construction in general).
        //
        // todo(sstelfox): should produce an error if an actor is added twice
        let actor_id = key.actor_id();

        if self.actor_settings.contains_key(&actor_id) {
            return Err(DriveAccessError::ActorAlreadyPresent);
        }

        let mut actor_settings = ActorSettings::new(key, access_mask);

        if access_mask.has_data_key() {
            let key = self
                .data_key
                .as_ref()
                .ok_or(DriveAccessError::PermissionEscalation)?;

            actor_settings
                .grant_data_key(rng, key)
                .map_err(DriveAccessError::GrantFailed)?;
        }

        if access_mask.has_filesystem_key() {
            let key = self
                .filesystem_key
                .as_ref()
                .ok_or(DriveAccessError::PermissionEscalation)?;

            actor_settings
                .grant_filesystem_key(rng, key)
                .map_err(DriveAccessError::GrantFailed)?;
        }

        if access_mask.has_maintenance_key() {
            let key = self
                .maintenance_key
                .as_ref()
                .ok_or(DriveAccessError::PermissionEscalation)?;

            actor_settings
                .grant_maintenance_key(rng, key)
                .map_err(DriveAccessError::GrantFailed)?;
        }

        self.actor_settings.insert(actor_id, actor_settings);

        Ok(())
    }

    /// This removes an actor's access by marking it as historical. The key is intentionally kept
    /// around to allow validation of signatures generated by this Actor as long as there are
    /// references to those signatures are present.
    ///
    /// The actor needs to have the correct permissions to make these changes in a way that other
    /// actors will accept. Specifically they need to meet the following requirements:
    ///
    /// * An actor CAN NOT remove themselves from the drive as they would be unable to sign and
    ///   distribute the update once their key has been marked as historical.
    /// * An actor with the protected flag MUST have its protected flag removed before it can have
    ///   its access revoked.
    /// * An actor MUST be marked as an owner to remove another owner from the drive.
    /// * An actor MUST have sufficient permissions to record the change to the filesystem, which
    ///   in this case only requires access to the maintenance key.
    ///
    /// An intentional consequence of being unable to remove yourself, and requiring an owner to
    /// change other owner keys is that there is guaranteed to always be at least one owner key
    /// remaining after this operation.
    pub fn remove_actor(
        &mut self,
        current_key: &SigningKey,
        actor_id: &ActorId,
    ) -> Result<(), DriveAccessError> {
        let current_actor_id = current_key.verifying_key().actor_id();
        if &current_actor_id == actor_id {
            return Err(DriveAccessError::SelfProtected);
        }

        let current_permissions =
            self.active_actor_access(&current_actor_id)
                .ok_or(DriveAccessError::AccessDenied(
                    "current actor has no access",
                ))?;

        let target_actor = self
            .actor_settings
            .get_mut(actor_id)
            .ok_or(DriveAccessError::UnknownActorId(*actor_id))?;

        if target_actor.access().is_protected() {
            return Err(DriveAccessError::AccessDenied(
                "protected keys can't be removed",
            ));
        }

        if !(current_permissions.has_maintenance_key()) {
            return Err(DriveAccessError::AccessDenied(
                "must be able to record changes to remove an actor",
            ));
        }

        if target_actor.access().is_owner() && !current_permissions.is_owner() {
            return Err(DriveAccessError::AccessDenied(
                "only owners can remove owner keys",
            ));
        }

        target_actor.access_mut().set_historical(true);

        Ok(())
    }

    /// Returns all the available [`ActorSettings`] associated with the current drive instance
    /// sorted by each configured actor's [`ActorId`]. We ultimately want this sorted by [`KeyId`]
    /// for consistent encoding which requires sorting by [`KeyId`], by sorting on the full
    /// [`ActorId`] we ensure the order is consistent in the face of the desired and highly likely
    /// collisions of the [`KeyId`].
    pub(crate) fn sorted_actor_settings(&self) -> Vec<&ActorSettings> {
        let mut actors: Vec<(&ActorId, &ActorSettings)> = self.actor_settings.iter().collect();
        actors.sort_by(|(aid, _), (bid, _)| aid.cmp(bid));
        actors.into_iter().map(|(_, settings)| settings).collect()
    }

    /// Attempts to get a handle on the unlocked copies of any escrowed permission keys. The
    /// provided [`SigningKey`] is expected to already have been granted access. Unknown keys will
    /// produce an error when the unlock is attempted.
    pub fn unlock_keys(&mut self, actor_key: &SigningKey) -> Result<(), DriveAccessError> {
        let actor_id = actor_key.verifying_key().actor_id();

        let settings = self
            .actor_settings
            .get(&actor_id)
            .ok_or(DriveAccessError::UnknownActorId(actor_id))?;

        self.filesystem_key = settings
            .filesystem_key(actor_key)
            .map_err(DriveAccessError::UnlockFailed)?;

        self.data_key = settings
            .data_key(actor_key)
            .map_err(DriveAccessError::UnlockFailed)?;

        self.maintenance_key = settings
            .maintenance_key(actor_key)
            .map_err(DriveAccessError::UnlockFailed)?;

        Ok(())
    }

    pub const fn size() -> usize {
        KeyId::size() + ActorSettings::size()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveAccessError {
    #[error("access denied: {0}")]
    AccessDenied(&'static str),

    #[error("access mask was invalid: {0}")]
    InvalidAccessMask(#[from] AccessMaskError),

    #[error("attempted to add an actor that already has access")]
    ActorAlreadyPresent,

    #[error("failed to grant actor permission key: {0}")]
    GrantFailed(ActorSettingsError),

    #[error("attempted to grant permission key the actor doesn't have access to")]
    PermissionEscalation,

    #[error("unable to remove the current actor from the drive")]
    SelfProtected,

    #[error("unknown actor id: {}", .0.as_hex())]
    UnknownActorId(ActorId),

    #[error("failed to unlock permission keys: {0}")]
    UnlockFailed(ActorSettingsError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_empty_encoding_produces_error() {
        let mut rng = crate::utils::crypto_rng();

        let empty_access = DriveAccess {
            actor_settings: HashMap::new(),

            filesystem_key: None,
            data_key: None,
            maintenance_key: None,
        };

        let mut buffer = Vec::new();
        assert!(empty_access.encode(&mut rng, &mut buffer).await.is_err());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_single_encoding_roundtrips() {
        let mut rng = crate::utils::crypto_rng();
        let key = SigningKey::generate(&mut rng);
        let verifying_key = key.verifying_key();

        let access = DriveAccess::initialize(&mut rng, verifying_key).unwrap();

        let mut buffer = Vec::new();
        access.encode(&mut rng, &mut buffer).await.unwrap();

        let (remaining, parsed) = DriveAccess::parse(Stream::new(&buffer), 1, &key).unwrap();
        assert!(remaining.is_empty());

        for (actor_id, settings) in access.actor_settings.iter() {
            let parsed_settings = parsed.actor_settings.get(actor_id).unwrap();
            assert_eq!(settings.verifying_key(), parsed_settings.verifying_key());
            assert_eq!(settings.access(), parsed_settings.access());
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_multiple_actor_encoding_roundtrips() {
        let mut rng = crate::utils::crypto_rng();
        let key = SigningKey::generate(&mut rng);
        let verifying_key = key.verifying_key();

        let mut access = DriveAccess::initialize(&mut rng, verifying_key).unwrap();

        let second_key = SigningKey::generate(&mut rng);
        let second_verifying_key = second_key.verifying_key();

        let access_mask = AccessMaskBuilder::structural().build().unwrap();
        access
            .register_actor(&mut rng, second_verifying_key, access_mask)
            .unwrap();

        let mut buffer = Vec::new();
        access.encode(&mut rng, &mut buffer).await.unwrap();

        let (remaining, parsed) = DriveAccess::parse(Stream::new(&buffer), 2, &key).unwrap();
        assert!(remaining.is_empty());

        for (actor_id, settings) in access.actor_settings.iter() {
            let parsed_settings = parsed.actor_settings.get(actor_id).unwrap();
            assert_eq!(settings.verifying_key(), parsed_settings.verifying_key());
            assert_eq!(settings.access(), parsed_settings.access());
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_initial_key_has_correct_privileges() {
        let mut rng = crate::utils::crypto_rng();
        let key = SigningKey::generate(&mut rng);
        let verifying_key = key.verifying_key();

        let actor_id = verifying_key.actor_id();
        let access = DriveAccess::initialize(&mut rng, verifying_key).unwrap();

        assert!(access.has_read_access(&actor_id));
        assert!(access.has_write_access(&actor_id));
        assert!(access.has_data_access(&actor_id));

        assert!(access.is_owner(&actor_id));
        assert!(access.is_protected(&actor_id));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_cant_remove_self() {
        let mut rng = crate::utils::crypto_rng();
        let key = SigningKey::generate(&mut rng);
        let verifying_key = key.verifying_key();

        let actor_id = verifying_key.actor_id();
        let mut access = DriveAccess::initialize(&mut rng, verifying_key).unwrap();

        assert!(matches!(
            access.remove_actor(&key, &actor_id).unwrap_err(),
            DriveAccessError::SelfProtected
        ));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_only_owner_can_remove_owner_keys() {
        let mut rng = crate::utils::crypto_rng();

        let actor1_key = SigningKey::generate(&mut rng);
        let actor1_verifying_key = actor1_key.verifying_key();
        let actor1_id = actor1_verifying_key.actor_id();

        // actor1 implicitly gets owner privileges here
        let mut access = DriveAccess::initialize(&mut rng, actor1_verifying_key).unwrap();

        let actor2_key = SigningKey::generate(&mut rng);
        let actor2_verifying_key = actor2_key.verifying_key();

        // actor2 gets full access but is critically _not_ an owner
        let actor2_access_mask = AccessMaskBuilder::full_access().build().unwrap();
        access
            .register_actor(&mut rng, actor2_verifying_key, actor2_access_mask)
            .unwrap();

        let removal_error = access.remove_actor(&actor2_key, &actor1_id).unwrap_err();

        assert!(matches!(removal_error, DriveAccessError::AccessDenied(_)));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_removal_leaves_key_as_historical() {
        let mut rng = crate::utils::crypto_rng();

        let actor1_key = SigningKey::generate(&mut rng);
        let actor1_verifying_key = actor1_key.verifying_key();

        let mut access = DriveAccess::initialize(&mut rng, actor1_verifying_key).unwrap();

        let actor2_key = SigningKey::generate(&mut rng);
        let actor2_verifying_key = actor2_key.verifying_key();
        let actor2_id = actor2_verifying_key.actor_id();

        let actor2_access_mask = AccessMaskBuilder::full_access().build().unwrap();
        access
            .register_actor(&mut rng, actor2_verifying_key, actor2_access_mask)
            .unwrap();

        assert!(access.active_actor_access(&actor2_id).is_some());
        access.remove_actor(&actor1_key, &actor2_id).unwrap();
        assert!(access.active_actor_access(&actor2_id).is_none());

        let actor2_access = access.actor_access(&actor2_id).unwrap();
        assert!(actor2_access.is_historical());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_protected_keys_cant_be_removed() {
        let mut rng = crate::utils::crypto_rng();

        let actor1_key = SigningKey::generate(&mut rng);
        let actor1_verifying_key = actor1_key.verifying_key();
        let actor1_id = actor1_verifying_key.actor_id();

        // actor1 implicitly gets owner privileges here, which should be sufficient
        let mut access = DriveAccess::initialize(&mut rng, actor1_verifying_key).unwrap();

        let actor2_key = SigningKey::generate(&mut rng);
        let actor2_verifying_key = actor2_key.verifying_key();

        let actor2_access_mask = AccessMaskBuilder::full_access()
            .protected()
            .build()
            .unwrap();
        access
            .register_actor(&mut rng, actor2_verifying_key, actor2_access_mask)
            .unwrap();

        let removal_error = access.remove_actor(&actor2_key, &actor1_id).unwrap_err();

        assert!(matches!(removal_error, DriveAccessError::AccessDenied(_)));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_register_basic_actor() {
        let mut rng = crate::utils::crypto_rng();
        let actor1_key = SigningKey::generate(&mut rng);
        let actor1_verifying_key = actor1_key.verifying_key();

        let actor1_id = actor1_verifying_key.actor_id();
        let mut access = DriveAccess::initialize(&mut rng, actor1_verifying_key).unwrap();

        let actor2_key = SigningKey::generate(&mut rng);
        let actor2_verifying_key = actor2_key.verifying_key();
        let actor2_id = actor2_verifying_key.actor_id();
        let actor2_access_mask = AccessMaskBuilder::full_access().build().unwrap();

        access
            .register_actor(&mut rng, actor2_verifying_key, actor2_access_mask)
            .unwrap();

        // Original actor should still have access
        assert!(access.has_write_access(&actor1_id));

        // New actor should have access
        assert!(access.has_write_access(&actor2_id));
    }
}
