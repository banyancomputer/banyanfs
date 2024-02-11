use std::collections::HashMap;

use time::OffsetDateTime;

use crate::codec::filesystem::Permissions;
use crate::codec::Cid;
use crate::filesystem::{ActorId, ContentReference};

pub struct File {
    owner: ActorId,

    permissions: Permissions,
    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,

    custom_metadata: HashMap<String, String>,

    content: Vec<ContentReference>,
}

impl File {
    pub fn calculate_cid(&self) -> Cid {
        todo!()
    }
}
