use crate::{
    codec::{ParserResult, Stream},
    filesystem::nodes::NodeName,
};
use futures::io::AsyncWrite;
use std::collections::HashMap;

use crate::codec::{Cid, PermanentId};

pub(crate) type ChildMap = HashMap<NodeName, ChildMapEntry>;

pub(crate) struct ChildMapEntry {
    permanent_id: PermanentId,
    cid: Cid,
}

impl ChildMapEntry {
    pub fn new(permanent_id: PermanentId, cid: Cid) -> Self {
        Self { permanent_id, cid }
    }

    pub fn permanent_id(&self) -> &PermanentId {
        &self.permanent_id
    }

    pub fn cid(&self) -> &Cid {
        &self.cid
    }

    pub fn set_cid(&mut self, cid: Cid) {
        self.cid = cid;
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        Ok(self.permanent_id().encode(writer).await? + self.cid().encode(writer).await?)
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, id) = PermanentId::parse(input)?;
        let (input, cid) = Cid::parse(input)?;
        Ok((input, Self::new(id, cid)))
    }
}
