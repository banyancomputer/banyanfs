use async_std::sync::RwLock;
use std::sync::Arc;

use crate::prelude::Node;

use super::{
    meta::{Cid, PermanentId},
    InnerDrive, OperationError,
};
use crate::filesystem::nodes::NodeContext;

pub(crate) struct DriveContext {
    inner: Arc<RwLock<InnerDrive>>,
}

impl NodeContext for DriveContext {
    async fn node_size(&self, id: &PermanentId) -> Result<u64, OperationError> {
        let inner = self.inner.read().await;
        let node = inner.by_perm_id(&id)?;
        Ok(node.size())
    }
    async fn node_cid(&self, id: &PermanentId) -> Result<Cid, OperationError> {
        let inner = self.inner.read().await;
        let node = inner.by_perm_id(&id)?;
        node.cid().await
    }
    async fn mark_node_dirty(&self, id: &PermanentId) -> Result<(), OperationError> {
        let mut inner = self.inner.write().await;
        let node = inner.by_perm_id_mut(&id)?;
        node.notify_of_change().await;
        Ok(())
    }
}
