use std::collections::HashMap;

use crate::filesystem::{ActorId, ContentReference, Permissions};

pub struct File {
    content: Vec<ContentReference>,
    owner: ActorId,
    permissions: Permissions,
    custom_metadata: HashMap<String, String>,
}
