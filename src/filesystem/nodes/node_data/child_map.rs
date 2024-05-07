use crate::codec::{ParserResult, Stream};
use futures::{AsyncWrite, AsyncWriteExt};
use std::collections::HashMap;
use winnow::{binary::le_u64, Parser};

use crate::{
    codec::{Cid, PermanentId},
    prelude::nodes::NodeName,
};

pub(crate) type ChildMap = HashMap<NodeName, ChildMapEntry>;

pub(crate) struct ChildMapEntry {
    permanent_id: PermanentId,
    cid: Cid,
    size: u64,
}

impl ChildMapEntry {
    pub fn new(permanent_id: PermanentId, cid: Cid, size: u64) -> Self {
        Self {
            permanent_id,
            cid,
            size,
        }
    }

    pub fn permanent_id(&self) -> &PermanentId {
        &self.permanent_id
    }

    pub fn cid(&self) -> &Cid {
        &self.cid
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn set_cid(&mut self, cid: Cid) {
        self.cid = cid;
    }

    pub fn set_size(&mut self, size: u64) {
        self.size = size;
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut bytes_written = self.permanent_id().encode(writer).await?;
        bytes_written += self.cid().encode(writer).await?;
        let size_bytes = self.size().to_le_bytes();
        writer.write_all(&size_bytes).await?;
        bytes_written += size_bytes.len();
        Ok(bytes_written)
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, id) = PermanentId::parse(input)?;
        let (input, cid) = Cid::parse(input)?;
        let (input, size) = le_u64.parse_peek(input)?;
        Ok((input, Self::new(id, cid, size)))
    }
}
