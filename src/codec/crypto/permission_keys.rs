use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::number::streaming::le_u8;

use crate::codec::crypto::{AccessKey, AsymLockedAccessKey, SigningKey, VerifyingKey};
use crate::codec::header::KeyAccessSettings;
use crate::codec::ParserResult;

const KEY_PRESENT_BIT: u8 = 0b0000_0001;

#[derive(Clone, PartialEq, Eq)]
pub struct PermissionKeys {
    pub(crate) filesystem: Option<AccessKey>,
    pub(crate) data: Option<AccessKey>,
    pub(crate) maintenance: Option<AccessKey>,
}

impl PermissionKeys {
    pub async fn encode_for<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        key_access: &KeyAccessSettings,
        verifying_key: &VerifyingKey,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        tracing::trace!("permission encoding start");

        written_bytes += maybe_encode_key(
            rng,
            writer,
            verifying_key,
            key_access.has_filesystem_key(),
            self.filesystem.as_ref(),
        )
        .await?;

        written_bytes += maybe_encode_key(
            rng,
            writer,
            verifying_key,
            key_access.has_data_key(),
            self.data.as_ref(),
        )
        .await?;

        written_bytes += maybe_encode_key(
            rng,
            writer,
            verifying_key,
            key_access.has_maintenance_key(),
            self.maintenance.as_ref(),
        )
        .await?;

        Ok(written_bytes)
    }

    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        Self {
            filesystem: Some(AccessKey::generate(rng)),
            data: Some(AccessKey::generate(rng)),
            maintenance: Some(AccessKey::generate(rng)),
        }
    }

    pub fn parse<'a>(input: &'a [u8], unlock_key: &SigningKey) -> ParserResult<'a, Self> {
        let (input, filesystem) = maybe_parse_key(input)?;
        let filesystem = filesystem
            .map(|key| key.unlock(unlock_key))
            .transpose()
            .map_err(|_| {
                nom::Err::Failure(nom::error::make_error(input, nom::error::ErrorKind::Verify))
            })?;

        let (input, data) = maybe_parse_key(input)?;
        let data = data
            .map(|key| key.unlock(unlock_key))
            .transpose()
            .map_err(|_| {
                nom::Err::Failure(nom::error::make_error(input, nom::error::ErrorKind::Verify))
            })?;

        let (input, maintenance) = maybe_parse_key(input)?;
        let maintenance = maintenance
            .map(|key| key.unlock(unlock_key))
            .transpose()
            .map_err(|_| {
                nom::Err::Failure(nom::error::make_error(input, nom::error::ErrorKind::Verify))
            })?;

        let permission_keys = Self {
            filesystem,
            data,
            maintenance,
        };

        Ok((input, permission_keys))
    }

    pub const fn size() -> usize {
        3 + AsymLockedAccessKey::size() * 3
    }
}

impl std::fmt::Debug for PermissionKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PermissionKeys(fs:{}, d:{}, mt:{})",
            self.filesystem.is_some(),
            self.data.is_some(),
            self.maintenance.is_some()
        )
    }
}

pub async fn maybe_encode_key<W: AsyncWrite + Unpin + Send>(
    rng: &mut impl CryptoRngCore,
    writer: &mut W,
    verifying_key: &VerifyingKey,
    allowed: bool,
    access_key: Option<&AccessKey>,
) -> std::io::Result<usize> {
    let mut written_bytes = 0;

    match access_key {
        Some(key) if allowed => {
            writer.write_all(&[0x01]).await?;
            written_bytes += 1;

            let protected_key = key.lock_for(rng, verifying_key).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "failed to lock permission key")
            })?;
            written_bytes += protected_key.encode(writer).await?;
        }
        _ => {
            writer.write_all(&[0x00]).await?;
            written_bytes += 1;

            // Write out empty bytes matching the normal size of a key
            let empty_key = [0u8; AsymLockedAccessKey::size()];
            writer.write_all(&empty_key).await?;
            written_bytes += empty_key.len();
        }
    }

    Ok(written_bytes)
}

fn maybe_parse_key(input: &[u8]) -> ParserResult<Option<AsymLockedAccessKey>> {
    let (input, presence_flag) = le_u8(input)?;

    if presence_flag & KEY_PRESENT_BIT != 0 {
        let (input, key) = AsymLockedAccessKey::parse(input)?;
        Ok((input, Some(key)))
    } else {
        // still need to advance the input
        let (input, _blank) = take(AsymLockedAccessKey::size())(input)?;
        Ok((input, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::codec::header::KeyAccessSettingsBuilder;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_permission_keys_roundtrip() {
        let mut rng = crate::utils::crypto_rng();

        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        let original = PermissionKeys::generate(&mut rng);

        let mut buffer = Vec::new();

        let kas = KeyAccessSettingsBuilder::private()
            .set_owner()
            .with_all_access()
            .build();

        original
            .encode_for(&mut rng, &mut buffer, &kas, &verifying_key)
            .await
            .unwrap();

        let (remaining, parsed) = PermissionKeys::parse(&buffer, &signing_key).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(original, parsed);
    }
}
