use crate::{
    codec::{Cid, PermanentId},
    prelude::OperationError,
};

pub(crate) trait NodeContext {
    async fn node_size(&self, id: &PermanentId) -> Result<u64, OperationError>;
    async fn node_cid(&self, id: &PermanentId) -> Result<Cid, OperationError>;
    async fn mark_node_dirty(&self, id: &PermanentId) -> Result<(), OperationError>;
}
