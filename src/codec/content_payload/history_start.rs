use crate::codec::content_payload::{Cid, ContentOptions};

pub struct HistoryStart {
    // todo: replace with vector type when we have it
    _journal_start_vector: u32,
    _merkle_root_cid: Cid,

    _content_options: ContentOptions,
}
