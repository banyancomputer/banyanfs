use std::collections::HashMap;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

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
    permission_keys: Option<PermissionKeys>,
}

impl DriveAccess {
    /// Queries the drive instance for the specific actor's current permissions. If the actor
    /// doesn't have any permissions this will return None.
    pub fn actor_access(&self, actor_id: ActorId) -> Option<AccessMask> {
        self.actor_settings
            .get(&actor_id)
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

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let permission_keys = self.permission_keys.as_ref().ok_or(StdError::new(
            StdErrorKind::InvalidData,
            "no permission keys available for encoding",
        ))?;

        for settings in self.sorted_actor_settings().iter() {
            let verifying_key = settings.verifying_key();
            let key_id = verifying_key.key_id();

            written_bytes += key_id.encode(writer).await?;

            let reset_agent_version = self.current_actor_id == verifying_key.actor_id();
            written_bytes += settings.encode(writer, reset_agent_version).await?;

            let access = settings.access();
            written_bytes += permission_keys
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
    pub fn has_data_access(&self, actor_id: ActorId) -> bool {
        let access = match self.actor_settings.get(&actor_id) {
            Some(s) => s.access(),
            None => return false,
        };

        if access.is_historical() {
            return false;
        }

        access.has_filesystem_key() && access.has_data_key()
    }

    /// Checks whether the requested actor is able to read the the structure of the filesystem and
    /// the associated attributes with its contents. This does not check whether the actor is able
    /// to read data from files or associated data nodes. To check whether the Actor can read data
    /// as well you'll want to use [`DriveAccess::has_data_access`].
    ///
    /// Historical keys will always return false.
    pub fn has_read_access(&self, actor_id: ActorId) -> bool {
        let access = match self.actor_settings.get(&actor_id) {
            Some(s) => s.access(),
            None => return false,
        };

        if access.is_historical() {
            return false;
        }

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
    pub fn has_write_access(&self, actor_id: ActorId) -> bool {
        let access = match self.actor_settings.get(&actor_id) {
            Some(s) => s.access(),
            None => return false,
        };

        if access.is_historical() {
            return false;
        }

        access.has_filesystem_key() && access.has_data_key() && access.has_maintenance_key()
    }

    /// Create a new DriveAccess instance with the provided actor as an owner with full
    /// permissions.
    pub(crate) fn initialize(rng: &mut impl CryptoRngCore, verifying_key: VerifyingKey) -> Self {
        let current_actor_id = verifying_key.actor_id();

        let mut access = Self {
            current_actor_id,
            actor_settings: HashMap::new(),
            permission_keys: Some(PermissionKeys::generate(rng)),
        };

        let access_mask = AccessMaskBuilder::full_access()
            .set_owner()
            .set_protected()
            .build();
        access.register_actor(verifying_key, access_mask);

        access
    }

    pub(crate) fn new(current_actor_id: ActorId) -> Self {
        Self {
            current_actor_id,
            actor_settings: HashMap::new(),
            permission_keys: None,
        }
    }

    pub fn parse<'a>(
        input: Stream<'a>,
        key_count: u8,
        signing_key: &SigningKey,
    ) -> ParserResult<'a, Self> {
        let mut actor_settings = HashMap::new();
        let mut permission_keys = None;

        let mut buf_slice = input;

        for _ in 0..key_count {
            let (i, key_id) = KeyId::parse(buf_slice).map_err(|_| {
                winnow::error::ErrMode::Cut(winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Verify,
                ))
            })?;
            buf_slice = i;

            let (i, settings) = ActorSettings::parse(buf_slice).map_err(|_| {
                winnow::error::ErrMode::Cut(winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Verify,
                ))
            })?;
            buf_slice = i;

            let verifying_key = settings.verifying_key();
            let actor_id = verifying_key.actor_id();
            actor_settings.insert(actor_id, settings);

            if key_id == verifying_key.key_id() {
                match PermissionKeys::parse(buf_slice, signing_key) {
                    Ok((i, keys)) => {
                        permission_keys = Some(keys);
                        buf_slice = i;
                        continue;
                    }
                    Err(err) => tracing::error!("failed to access permission keys: {err}"),
                };
            }
        }

        if permission_keys.is_none() {
            tracing::warn!("no matching permission keys found for provided key");
        }

        let current_actor_id = signing_key.actor_id();

        let drive_access = Self {
            current_actor_id,
            actor_settings,
            permission_keys,
        };

        Ok((buf_slice, drive_access))
    }

    pub fn permission_keys(&self) -> Option<&PermissionKeys> {
        self.permission_keys.as_ref()
    }

    /// Adds a new actor (via their [`VerifyingKey`]) to the drive with the provided permissions.
    pub fn register_actor(&mut self, key: VerifyingKey, access_mask: AccessMask) {
        // todo(sstelfox): need to add a check that prevents a user from granting privileges beyond
        // their own (they couldn't anyways as they don't have access to the symmetric keys
        // necessary, but we should prevent invalid construction in general).
        let actor_id = key.actor_id();
        let actor_settings = ActorSettings::new(key, access_mask);

        self.actor_settings.insert(actor_id, actor_settings);
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
