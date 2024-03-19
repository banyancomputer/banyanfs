use async_trait::async_trait;

use crate::codec::Cid;

// todo: move this under src/stores
#[async_trait(?Send)]
pub trait DataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError>;

    async fn remove(&mut self, cid: Cid, recusrive: bool) -> Result<Option<u64>, DataStoreError>;

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError>;

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        immediate: bool,
    ) -> Result<(), DataStoreError>;
}

#[async_trait(?Send)]
pub trait SyncableDataStore: DataStore {
    type Tracker: SyncTracker;

    async fn sync(&mut self) -> Result<(), DataStoreError>;

    fn tracker(&self) -> &Self::Tracker;

    fn tracker_mut(&mut self) -> &mut Self::Tracker;

    async fn store_sync(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), DataStoreError> {
        self.store(cid, data, true).await
    }

    async fn unsynced_data_size(&self) -> Result<u64, DataStoreError> {
        self.tracker().tracked_size().await
    }
}

#[async_trait(?Send)]
pub trait SyncTracker {
    async fn track(&mut self, cid: Cid, size: u64) -> Result<(), DataStoreError>;

    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError>;

    async fn tracked_size(&self) -> Result<u64, DataStoreError>;

    async fn untrack(&mut self, cid: Cid) -> Result<(), DataStoreError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DataStoreError {
    #[error("failed to retrieve block")]
    LookupFailure,

    #[error("failed to store block")]
    StoreFailure,

    #[error("block not available in this data store: {0:?}")]
    UnknownBlock(Cid),
}
