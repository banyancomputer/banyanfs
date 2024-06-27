// Still needed for the journal entries
#![allow(dead_code)]

use futures::io::AsyncWrite;

use crate::codec::meta::{Cid, VectorClock};
use crate::codec::{ParserResult, Stream};

#[derive(Clone, Debug, PartialEq)]
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

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, merkle_root_cid) = Cid::parse(input)?;
        let (input, vector) = VectorClock::parse(input)?;

        let journal_checkpoint = JournalCheckpoint {
            merkle_root_cid,
            vector,
        };

        Ok((input, journal_checkpoint))
    }

    pub const fn size() -> usize {
        Cid::size() + VectorClock::size()
    }
}

#[cfg(test)]
mod tests {
    use winnow::Partial;

    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test(async))]
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    async fn test_user_agent_roundtrip() {
        let checkpoint = JournalCheckpoint::initialize();

        let mut buffer = Vec::with_capacity(JournalCheckpoint::size());
        checkpoint
            .encode(&mut buffer)
            .await
            .expect("encoding success");

        let partial = Partial::new(buffer.as_slice());
        let (remaining, parsed) = JournalCheckpoint::parse(partial).expect("round trip");

        assert!(remaining.is_empty());
        assert_eq!(checkpoint, parsed);
    }
}
