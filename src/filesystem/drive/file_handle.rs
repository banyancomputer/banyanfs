use std::sync::Arc;

use async_std::sync::RwLock;

use crate::filesystem::nodes::NodeId;

use crate::codec::crypto::SigningKey;
use crate::filesystem::drive::InnerDrive;

pub struct FileHandle {
    _current_key: Arc<SigningKey>,
    _node_id: NodeId,
    _inner: Arc<RwLock<InnerDrive>>,
}

impl FileHandle {}
