use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u64;
use nom::IResult;

use crate::codec::{AsyncEncodable, Cid};

pub struct HistoryEnd {
    // todo: replace with vector type when we have it
    journal_end_vector: u64,
    merkle_root_cid: Cid,
}

impl HistoryEnd {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, journal_end_vector) = le_u64(input)?;
        let (input, merkle_root_cid) = Cid::parse(input)?;

        let history_end = HistoryEnd {
            journal_end_vector,
            merkle_root_cid,
        };

        Ok((input, history_end))
    }
}

#[async_trait]
impl AsyncEncodable for HistoryEnd {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let journal_end_vector_bytes = self.journal_end_vector.to_le_bytes();
        writer.write_all(&journal_end_vector_bytes).await?;
        written_bytes += journal_end_vector_bytes.len();

        written_bytes += self.merkle_root_cid.encode(writer).await?;

        Ok(written_bytes)
    }
}
