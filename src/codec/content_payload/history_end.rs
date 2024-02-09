use crate::codec::content_payload::Cid;

pub struct HistoryEnd {
    // todo: replace with vector type when we have it
    _journal_end_vector: u32,
    _merkle_root_cid: Cid,
}

impl HistoryEnd {
    pub fn parse(_input: &[u8]) -> nom::IResult<&[u8], Self> {
        todo!()
    }
}
