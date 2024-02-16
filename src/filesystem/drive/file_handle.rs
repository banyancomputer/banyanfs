use std::sync::Arc;

use async_std::sync::RwLock;
use elliptic_curve::rand_core::CryptoRngCore;
use tracing::{debug, instrument, trace, Instrument, Level};

use crate::codec::*;
use crate::filesystem::nodes::{Node, NodeId, NodeKind};

use crate::codec::crypto::SigningKey;
use crate::filesystem::drive::{InnerDrive, WalkState};

pub struct FileHandle {
    current_key: Arc<SigningKey>,
    node_id: NodeId,
    inner: Arc<RwLock<InnerDrive>>,
}

impl FileHandle {}
