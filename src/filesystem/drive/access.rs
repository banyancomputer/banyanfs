use std::collections::HashMap;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use elliptic_curve::rand_core::CryptoRngCore;
use futures::io::AsyncWrite;

use crate::codec::crypto::{KeyId, PermissionKeys, SigningKey, VerifyingKey};
use crate::codec::header::AccessMask;
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
    pub fn actor_access(&self, actor_id: ActorId) -> Option<AccessMask> {
        self.actor_settings
            .get(&actor_id)
            .map(|settings| settings.access())
    }

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

    pub fn init_private(rng: &mut impl CryptoRngCore, current_actor_id: ActorId) -> Self {
        Self {
            current_actor_id,
            actor_settings: HashMap::new(),
            permission_keys: Some(PermissionKeys::generate(rng)),
        }
    }

    pub fn new(current_actor_id: ActorId) -> Self {
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

    pub fn register_actor(&mut self, key: VerifyingKey, access_mask: AccessMask) {
        let actor_id = key.actor_id();
        let actor_settings = ActorSettings::new(key, access_mask);

        self.actor_settings.insert(actor_id, actor_settings);
    }

    pub fn sorted_actor_settings(&self) -> Vec<&ActorSettings> {
        let mut actors: Vec<(&ActorId, &ActorSettings)> = self.actor_settings.iter().collect();
        actors.sort_by(|(aid, _), (bid, _)| aid.key_id().cmp(&bid.key_id()));
        actors.into_iter().map(|(_, settings)| settings).collect()
    }

    pub const fn size() -> usize {
        KeyId::size() + ActorSettings::size() + PermissionKeys::size()
    }
}
