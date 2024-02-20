use crate::codec::header::ContentOptions;
use crate::codec::meta::JournalCheckpoint;
use crate::filesystem::DriveAccess;

#[derive(Debug)]
pub(crate) struct ContentContext {
    pub(crate) history_start: JournalCheckpoint,
    pub(crate) content_options: ContentOptions,
    pub(crate) access: DriveAccess,
}
