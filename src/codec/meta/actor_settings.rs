use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{binary::le_u8, token::take, Parser};

use crate::codec::{
    crypto::{AccessKey, AsymLockedAccessKey, AsymLockedAccessKeyError, SigningKey, VerifyingKey},
    header::AccessMask,
    meta::{UserAgent, VectorClockActorSnapshot},
    ParserResult, Stream,
};

const KEY_PRESENT_BIT: u8 = 0b0000_0001;

#[derive(Clone, Debug)]
pub struct ActorSettings {
    verifying_key: VerifyingKey,
    vector_clock: VectorClockActorSnapshot,

    access_mask: AccessMask,
    filesystem_key: Option<AsymLockedAccessKey>,
    data_key: Option<AsymLockedAccessKey>,
    maintenance_key: Option<AsymLockedAccessKey>,

    // todo(sstelfox): this would be a breaking change but this should be optional. When granting
    // access to a new key, we only know what user agent the _current_ user is not the one being
    // granted access. Recording our own as the user's agent is an error.
    user_agent: UserAgent,
}

impl ActorSettings {
    pub fn access(&self) -> AccessMask {
        self.access_mask
    }

    pub(crate) fn access_mut(&mut self) -> &mut AccessMask {
        &mut self.access_mask
    }

    pub fn clear_data_key(&mut self) {
        self.data_key = None;
        self.access_mask.set_data_key_present(false);
    }

    pub fn clear_filesystem_key(&mut self) {
        self.filesystem_key = None;
        self.access_mask.set_filesystem_key_present(false);
    }

    pub fn clear_maintenance_key(&mut self) {
        self.data_key = None;
        self.access_mask.set_data_key_present(false);
    }

    pub fn data_key(
        &self,
        actor_key: &SigningKey,
    ) -> Result<Option<AccessKey>, ActorSettingsError> {
        if !self.access_mask.has_data_key() {
            return Ok(None);
        }

        let locked_key = self
            .data_key
            .as_ref()
            .ok_or(ActorSettingsError::ExpectedKeyMissing)?;

        let open_key = locked_key
            .unlock(actor_key)
            .map_err(ActorSettingsError::UnlockFailed)?;

        Ok(Some(open_key))
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += self.verifying_key.encode(writer).await?;
        written_bytes += self.vector_clock.encode(writer).await?;
        written_bytes += self.access_mask.encode(writer).await?;
        written_bytes += self.user_agent().encode(writer).await?;

        written_bytes += encode_optional_key(writer, &self.filesystem_key).await?;
        written_bytes += encode_optional_key(writer, &self.data_key).await?;
        written_bytes += encode_optional_key(writer, &self.maintenance_key).await?;

        Ok(written_bytes)
    }

    pub fn filesystem_key(
        &self,
        actor_key: &SigningKey,
    ) -> Result<Option<AccessKey>, ActorSettingsError> {
        if !self.access_mask.has_filesystem_key() {
            return Ok(None);
        }

        let locked_key = self
            .filesystem_key
            .as_ref()
            .ok_or(ActorSettingsError::ExpectedKeyMissing)?;

        let open_key = locked_key
            .unlock(actor_key)
            .map_err(ActorSettingsError::UnlockFailed)?;

        Ok(Some(open_key))
    }

    pub fn grant_data_key(
        &mut self,
        rng: &mut impl CryptoRngCore,
        key: &AccessKey,
    ) -> Result<(), ActorSettingsError> {
        let locked_key = key
            .lock_for(rng, &self.verifying_key)
            .map_err(|_| ActorSettingsError::KeyEscrowError)?;

        self.data_key = Some(locked_key);
        self.access_mask.set_data_key_present(true);

        Ok(())
    }

    pub fn grant_filesystem_key(
        &mut self,
        rng: &mut impl CryptoRngCore,
        key: &AccessKey,
    ) -> Result<(), ActorSettingsError> {
        let locked_key = key
            .lock_for(rng, &self.verifying_key)
            .map_err(|_| ActorSettingsError::KeyEscrowError)?;

        self.filesystem_key = Some(locked_key);
        self.access_mask.set_filesystem_key_present(true);

        Ok(())
    }

    pub fn grant_maintenance_key(
        &mut self,
        rng: &mut impl CryptoRngCore,
        key: &AccessKey,
    ) -> Result<(), ActorSettingsError> {
        let locked_key = key
            .lock_for(rng, &self.verifying_key)
            .map_err(|_| ActorSettingsError::KeyEscrowError)?;

        self.maintenance_key = Some(locked_key);
        self.access_mask.set_maintenance_key_present(true);

        Ok(())
    }

    pub fn maintenance_key(
        &self,
        actor_key: &SigningKey,
    ) -> Result<Option<AccessKey>, ActorSettingsError> {
        if !self.access_mask.has_data_key() {
            return Ok(None);
        }

        let locked_key = self
            .maintenance_key
            .as_ref()
            .ok_or(ActorSettingsError::ExpectedKeyMissing)?;

        let open_key = locked_key
            .unlock(actor_key)
            .map_err(ActorSettingsError::UnlockFailed)?;

        Ok(Some(open_key))
    }

    pub fn new(
        verifying_key: VerifyingKey,
        access_mask: AccessMask,
        vector_clock: VectorClockActorSnapshot,
    ) -> Self {
        // Should we test that the actor id associated with the verifying key matches the actor id of the vector clock?
        let user_agent = UserAgent::current();

        Self {
            verifying_key,
            vector_clock,

            access_mask,
            filesystem_key: None,
            data_key: None,
            maintenance_key: None,

            user_agent,
        }
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, verifying_key) = VerifyingKey::parse(input)?;
        let (input, vector_clock) = VectorClockActorSnapshot::parse(input)?;
        let (input, access_mask) = AccessMask::parse(input)?;
        let (input, user_agent) = UserAgent::parse(input)?;

        let (input, filesystem_key) = decode_optional_key(input)?;
        let (input, data_key) = decode_optional_key(input)?;
        let (input, maintenance_key) = decode_optional_key(input)?;

        let actor_settings = Self {
            verifying_key,
            vector_clock,

            access_mask,
            filesystem_key,
            data_key,
            maintenance_key,

            user_agent,
        };

        Ok((input, actor_settings))
    }

    pub const fn size() -> usize {
        VerifyingKey::size()
            + VectorClockActorSnapshot::size()
            + AccessMask::size()
            + UserAgent::size()
            + 3 * (1 + AsymLockedAccessKey::size())
    }

    pub fn update_user_agent(&mut self) {
        self.user_agent = UserAgent::current();
    }

    pub fn user_agent(&self) -> UserAgent {
        self.user_agent.clone()
    }

    pub fn vector_clock(&self) -> VectorClockActorSnapshot {
        self.vector_clock
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ActorSettingsError {
    #[error("failed to escrow access key for actor")]
    KeyEscrowError,

    #[error("actor has permissions to access a key that wasn't present")]
    ExpectedKeyMissing,

    #[error("access key failed to unlock with actor's key")]
    UnlockFailed(AsymLockedAccessKeyError),
}

fn decode_optional_key(input: Stream) -> ParserResult<Option<AsymLockedAccessKey>> {
    let (input, presence_flag) = le_u8.parse_peek(input)?;

    if cfg!(feature = "strict") && presence_flag != 0 && presence_flag != KEY_PRESENT_BIT {
        return Err(winnow::error::ErrMode::Cut(
            winnow::error::ParserError::from_error_kind(&input, winnow::error::ErrorKind::Verify),
        ));
    }

    if presence_flag & KEY_PRESENT_BIT != 0 {
        let (input, key) = AsymLockedAccessKey::parse(input)?;
        Ok((input, Some(key)))
    } else {
        // still need to advance the input
        let (input, _blank) = take(AsymLockedAccessKey::size()).parse_peek(input)?;
        Ok((input, None))
    }
}

async fn encode_optional_key<W: AsyncWrite + Unpin + Send>(
    writer: &mut W,
    key: &Option<AsymLockedAccessKey>,
) -> std::io::Result<usize> {
    let mut written_bytes = 0;

    match key {
        Some(key) => {
            writer.write_all(&[KEY_PRESENT_BIT]).await?;
            written_bytes += 1;

            written_bytes += key.encode(writer).await?;
        }
        None => {
            writer.write_all(&[0x00]).await?;
            written_bytes += 1;

            let empty_key = [0u8; AsymLockedAccessKey::size()];
            writer.write_all(&empty_key).await?;
            written_bytes += AsymLockedAccessKey::size();
        }
    }

    Ok(written_bytes)
}
