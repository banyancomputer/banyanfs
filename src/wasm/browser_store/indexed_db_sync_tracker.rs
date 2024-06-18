use async_trait::async_trait;

use crate::codec::meta::Cid;
use crate::stores::{DataStoreError, SyncTracker};

pub struct IndexedDbSyncTracker;

#[async_trait(?Send)]
impl SyncTracker for IndexedDbSyncTracker {
    async fn clear_deleted(&mut self) -> Result<(), DataStoreError> {
        todo!()
    }

    async fn delete(&mut self, _cid: Cid) -> Result<(), DataStoreError> {
        todo!()
    }

    async fn deleted_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        todo!()
    }

    async fn track(&mut self, _cid: Cid, _size: u64) -> Result<(), DataStoreError> {
        todo!()
    }

    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        todo!()
    }

    async fn tracked_size(&self) -> Result<u64, DataStoreError> {
        todo!()
    }

    async fn untrack(&mut self, _cid: Cid) -> Result<(), DataStoreError> {
        todo!()
    }
}
