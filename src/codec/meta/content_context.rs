use crate::codec::header::ContentOptions;
use crate::codec::meta::JournalCheckpoint;

#[derive(Debug)]
pub(crate) struct ContentContext {
    pub(crate) history_start: JournalCheckpoint,
    pub(crate) content_options: ContentOptions,
    pub(crate) permission_controls: Vec<()>,
}
