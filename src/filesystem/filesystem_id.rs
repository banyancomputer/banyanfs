use std::time::SystemTime;

use futures::{AsyncWrite, AsyncWriteExt};
use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::sequence::tuple;
use rand::{Rng, RngCore};
use uuid::{NoContext, Timestamp, Uuid};

use crate::codec::AsyncEncodable;

const ID_LENGTH: usize = 16;

pub struct FilesystemId([u8; ID_LENGTH]);

impl FilesystemId {
    pub fn generate(rng: &mut impl RngCore) -> Self {
        let ts = Timestamp::now(NoContext);
        let uuid = Uuid::new_v7(ts);
        Self(uuid.to_bytes_le())
    }

    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, id_bytes) = take(ID_LENGTH)(input)?;

        // All zeros and all ones are disallowed, this isn't actually harmful though so we'll only
        // perform this check in strict mode.
        if cfg!(feature = "strict")
            && (id_bytes.iter().all(|&b| b == 0x00) || id_bytes.iter().all(|&b| b == 0xff))
        {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)));
        }

        // todo(sstelfox): parse into an actually UUID, validate the version, probably store the
        // UUID instead of the bytes.

        let mut bytes = [0u8; ID_LENGTH];
        bytes.copy_from_slice(id_bytes);

        Ok((remaining, Self(bytes)))
    }

    pub const fn size() -> usize {
        ID_LENGTH
    }
}

#[async_trait::async_trait]
impl AsyncEncodable for FilesystemId {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> std::io::Result<usize> {
        writer.write_all(&self.0).await?;
        Ok(start_pos + self.0.len())
    }
}
