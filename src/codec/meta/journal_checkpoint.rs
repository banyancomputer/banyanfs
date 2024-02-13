use nom::IResult;

use crate::codec::Cid;
use crate::filesystem::VectorClock;

#[derive(Debug)]
pub struct JournalCheckpoint {
    merkle_root_cid: Cid,
    vector: VectorClock,
}

impl JournalCheckpoint {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, merkle_root_cid) = Cid::parse(input)?;
        let (input, vector) = VectorClock::parse(input)?;

        let journal_checkpoint = JournalCheckpoint {
            merkle_root_cid,
            vector,
        };

        Ok((input, journal_checkpoint))
    }
}
