use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u64;
use nom::IResult;

use crate::codec::content_payload::ContentOptions;
use crate::codec::{AsyncEncodable, Cid};

pub struct HistoryStart {
    // todo: replace with vector type when we have it
    journal_start_vector: u64,
    merkle_root_cid: Cid,

    content_options: ContentOptions,
}

impl HistoryStart {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, journal_start_vector) = le_u64(input)?;
        let (input, cid) = Cid::parse(input)?;
        let (input, content_options) = ContentOptions::parse(input)?;

        let history_start = HistoryStart {
            journal_start_vector,
            merkle_root_cid: cid,
            content_options,
        };

        Ok((input, history_start))
    }

    pub const fn size() -> usize {
        8 + Cid::size() + ContentOptions::size()
    }
}

#[async_trait]
impl AsyncEncodable for HistoryStart {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        let journal_start_vector_bytes = self.journal_start_vector.to_le_bytes();
        writer.write_all(&journal_start_vector_bytes).await?;
        written_bytes += journal_start_vector_bytes.len();

        written_bytes += self.merkle_root_cid.encode(writer).await?;
        written_bytes += self.content_options.encode(writer).await?;

        Ok(written_bytes)
    }
}
