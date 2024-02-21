use futures::io::AsyncWrite;

use crate::codec::meta::{Cid, VectorClock};
use crate::codec::ParserResult;

#[derive(Clone, Debug)]
pub struct JournalCheckpoint {
    merkle_root_cid: Cid,
    vector: VectorClock,
}

impl JournalCheckpoint {
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        written_bytes += self.merkle_root_cid.encode(writer).await?;
        written_bytes += self.vector.encode(writer).await?;

        Ok(written_bytes)
    }

    pub(crate) fn initialize() -> Self {
        JournalCheckpoint {
            merkle_root_cid: Cid::IDENTITY,
            vector: VectorClock::initialize(),
        }
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, merkle_root_cid) = Cid::parse(input)?;
        let (input, vector) = VectorClock::parse(input)?;

        let journal_checkpoint = JournalCheckpoint {
            merkle_root_cid,
            vector,
        };

        Ok((input, journal_checkpoint))
    }
}
