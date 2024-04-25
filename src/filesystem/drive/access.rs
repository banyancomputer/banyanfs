use std::collections::HashMap;

use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::AsyncWrite;

use crate::codec::crypto::{KeyId, PermissionKeys, SigningKey, VerifyingKey};
use crate::codec::header::{AccessMask, AccessMaskBuilder};
use crate::codec::{ActorId, ActorSettings, ParserResult, Stream};

/// [`DriveAccess`] maintains a mapping of [`ActorId`] instances to their available permissions
/// within the drive itself. When loaded this holds on to copies of any of the general keys the
/// current actor has access to.
///
/// Access within the drive is broken up based on which internal symmetric keys the current user
/// has access to. These keys are held within the [`PermissionKeys`] struct and consist of a
/// filesystem key (which grants access to read the structure and metadata of the filesystem), a
/// data key which protects the per-file encryption key needed to access the data contained in any
/// blocks stored), and a maintenance key (which is only used to read block lifecycle information
/// found in different metadata version syncs).
#[derive(Clone, Debug)]
pub struct DriveAccess {
    current_actor_id: ActorId,
    actor_settings: HashMap<ActorId, ActorSettings>,
    permission_keys: PermissionKeys,
}

impl DriveAccess {
    /// Similar to [`DriveAccess::actor_access`], this performs an additional check to reject
    /// historical keys limiting queries to the common case of keys that should currently have some
    /// level of access.
    pub fn active_actor_access(&self, actor_id: &ActorId) -> Option<AccessMask> {
        let access = self
            .actor_settings
            .get(actor_id)
            .map(|settings| settings.access());

        match access {
            Some(a) if a.is_historical() => None,
            _ => access,
        }
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

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        if self.actor_settings.len() == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "drive must have at least one registered actor",
            ));
        }

        for settings in self.sorted_actor_settings().iter() {
            let verifying_key = settings.verifying_key();
            let key_id = verifying_key.key_id();

            written_bytes += key_id.encode(writer).await?;

            let reset_agent_version = self.current_actor_id == verifying_key.actor_id();
            written_bytes += settings.encode(writer, reset_agent_version).await?;

            let access = settings.access();
            written_bytes += self
                .permission_keys
                .encode_for(rng, writer, &access, &verifying_key)
                .await?;
        }

        Ok(written_bytes)
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
        let access = match self.active_actor_access(&actor_id) {
            Some(a) => a,
            None => return false,
        };

        access.has_filesystem_key() && access.has_data_key()
    }

    /// Checks whether the requested actor is able to read the the structure of the filesystem and
    /// the associated attributes with its contents. This does not check whether the actor is able
    /// to read data from files or associated data nodes. To check whether the Actor can read data
    /// as well you'll want to use [`DriveAccess::has_data_access`].
    ///
    /// Historical keys will always return false.
    pub fn has_read_access(&self, actor_id: &ActorId) -> bool {
        let access = match self.active_actor_access(&actor_id) {
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
        let access = match self.active_actor_access(&actor_id) {
            Some(a) => a,
            None => return false,
        };

        access.has_filesystem_key() && access.has_data_key() && access.has_maintenance_key()
    }

    /// Create a new DriveAccess instance with the provided actor as an owner with full
    /// permissions.
    pub(crate) fn initialize(rng: &mut impl CryptoRngCore, verifying_key: VerifyingKey) -> Self {
        let current_actor_id = verifying_key.actor_id();

        let mut access = Self {
            current_actor_id,
            actor_settings: HashMap::new(),
            permission_keys: PermissionKeys::generate(rng),
        };

        let access_mask = AccessMaskBuilder::full_access()
            .set_owner()
            .set_protected()
            .build();

        access.register_actor(verifying_key, access_mask);

        access
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
            None => return false,
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
            None => return false,
        }
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
        let mut permission_keys = None;
        let mut buf_slice = input;

        for _ in 0..key_count {
            let (i, key_id) = KeyId::parse(buf_slice)?;
            buf_slice = i;

            let (i, settings) = ActorSettings::parse(buf_slice)?;
            buf_slice = i;

            let verifying_key = settings.verifying_key();
            let actor_id = verifying_key.actor_id();
            actor_settings.insert(actor_id, settings);

            if key_id == verifying_key.key_id() {
                let (i, keys) = PermissionKeys::parse(buf_slice, signing_key)?;
                permission_keys = Some(keys);
                buf_slice = i;
            } else {
                todo!("need to store the encoded permission keys for re-encoding in case we don't have access to all of them");
                //let (i, _) = take(PermissionKeys::size()).parse_peek(buf_slice)?;
                //buf_slice = i;
            }
        }

        let permission_keys = match permission_keys {
            Some(pk) => pk,
            None => {
                tracing::warn!("no matching permission keys found for provided key");

                return Err(winnow::error::ErrMode::Cut(
                    winnow::error::ParserError::from_error_kind(
                        &buf_slice,
                        winnow::error::ErrorKind::Verify,
                    ),
                ));
            }
        };

        let current_actor_id = signing_key.actor_id();

        let drive_access = Self {
            current_actor_id,
            actor_settings,
            permission_keys,
        };

        Ok((buf_slice, drive_access))
    }

    /// Retrieves the unencrypted symmetric keys the current actor has available used to access the
    /// various permission levels. A user must have access to a permission key to grant that
    /// permission to others.
    pub fn permission_keys(&self) -> &PermissionKeys {
        &self.permission_keys
    }

    /// Adds a new actor (via their [`VerifyingKey`]) to the drive with the provided permissions.
    pub fn register_actor(&mut self, key: VerifyingKey, access_mask: AccessMask) {
        // todo(sstelfox): need to add a check that prevents a user from granting privileges beyond
        // their own (they couldn't anyways as they don't have access to the symmetric keys
        // necessary, but we should prevent invalid construction in general).
        //
        // todo(sstelfox): should produce an error if an actor is added twice
        //
        // todo(sstelfox): need to grant the actor access to the permission keys
        let actor_id = key.actor_id();
        let actor_settings = ActorSettings::new(key, access_mask);

        self.actor_settings.insert(actor_id, actor_settings);
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
    pub fn remove_actor(&mut self, actor_id: &ActorId) -> Result<(), DriveAccessError> {
        if &self.current_actor_id == actor_id {
            return Err(DriveAccessError::SelfProtected);
        }

        let mut permissions = self.active_actor_access(&self.current_actor_id).ok_or(
            DriveAccessError::AccessDenied("current actor has no access"),
        )?;

        let target_actor = self
            .actor_settings
            .get_mut(actor_id)
            .ok_or_else(|| DriveAccessError::UnknownActorId(actor_id.clone()))?;

        if target_actor.access().is_protected() {
            return Err(DriveAccessError::AccessDenied(
                "protected keys can't be removed",
            ));
        }

        if !(permissions.has_maintenance_key()) {
            return Err(DriveAccessError::AccessDenied(
                "must be able to record changes to remove an actor",
            ));
        }

        if target_actor.access().is_owner() && !permissions.is_owner() {
            return Err(DriveAccessError::AccessDenied(
                "only owners can remove owner keys",
            ));
        }

        permissions.set_historical(true);

        Ok(())
    }

    /// Returns all the available [`ActorSettings`] associated with the current drive instance
    /// sorted by each configured actor's [`KeyId`]. This is used for consistency in encoding
    /// drives and provides a level of obscurity over the order that keys were added to a drive.
    pub fn sorted_actor_settings(&self) -> Vec<&ActorSettings> {
        let mut actors: Vec<(&ActorId, &ActorSettings)> = self.actor_settings.iter().collect();
        actors.sort_by(|(aid, _), (bid, _)| aid.key_id().cmp(&bid.key_id()));
        actors.into_iter().map(|(_, settings)| settings).collect()
    }

    pub const fn size() -> usize {
        KeyId::size() + ActorSettings::size() + PermissionKeys::size()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DriveAccessError {
    #[error("access denied: {0}")]
    AccessDenied(&'static str),

    #[error("unable to remove the current actor from the drive")]
    SelfProtected,

    #[error("unknown actor id: {}", .0.as_hex())]
    UnknownActorId(ActorId),
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
        let current_actor_id = ActorId::arbitrary(&mut rng);

        let empty_access = DriveAccess {
            current_actor_id,
            actor_settings: HashMap::new(),
            permission_keys: PermissionKeys {
                filesystem: None,
                data: None,
                maintenance: None,
            },
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

        let access = DriveAccess::initialize(&mut rng, verifying_key);

        let mut buffer = Vec::new();
        access.encode(&mut rng, &mut buffer).await.unwrap();

        let parsed = DriveAccess::parse(Stream::new(&buffer), 1, &key).unwrap().1;

        assert_eq!(access.current_actor_id, parsed.current_actor_id);

        for (key, settings) in access.actor_settings.iter() {
            let parsed_settings = parsed.actor_settings.get(key).unwrap();
            assert_eq!(settings.verifying_key(), parsed_settings.verifying_key());
            assert_eq!(settings.access(), parsed_settings.access());
        }

        assert_eq!(access.permission_keys, parsed.permission_keys);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[ignore]
    async fn test_multiple_actor_encoding_roundtrips() {
        todo!()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_initial_key_has_correct_privileges() {
        let mut rng = crate::utils::crypto_rng();
        let key = SigningKey::generate(&mut rng);
        let verifying_key = key.verifying_key();

        let actor_id = verifying_key.actor_id();
        let access = DriveAccess::initialize(&mut rng, verifying_key);

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
        let mut access = DriveAccess::initialize(&mut rng, verifying_key);

        assert!(matches!(
            access.remove_actor(&actor_id).unwrap_err(),
            DriveAccessError::SelfProtected
        ));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[ignore]
    async fn test_only_owner_can_remove_owner_keys() {
        todo!()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[ignore]
    async fn test_protected_keys_cant_be_removed() {
        todo!()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_register_basic_actor() {
        let mut rng = crate::utils::crypto_rng();
        let actor1_key = SigningKey::generate(&mut rng);
        let actor1_verifying_key = actor1_key.verifying_key();

        let actor1_id = actor1_verifying_key.actor_id();
        let mut access = DriveAccess::initialize(&mut rng, actor1_verifying_key);

        let actor2_key = SigningKey::generate(&mut rng);
        let actor2_verifying_key = actor2_key.verifying_key();
        let actor2_id = actor2_verifying_key.actor_id();
        let actor2_access_mask = AccessMaskBuilder::full_access().build();

        access.register_actor(actor2_verifying_key, actor2_access_mask);

        // Original actor should still have access
        assert!(access.has_write_access(&actor1_id));

        // New actor should have access
        assert!(access.has_write_access(&actor2_id));
    }
}
