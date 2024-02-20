use crate::codec::meta::{Cid, VectorClock};
use crate::codec::ParserResult;

#[derive(Debug)]
pub struct JournalCheckpoint {
    merkle_root_cid: Cid,
    vector: VectorClock,
}

impl JournalCheckpoint {
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
