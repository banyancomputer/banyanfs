use async_trait::async_trait;
use url::Url;

use crate::codec::Cid;

#[async_trait(?Send)]
pub trait DataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError>;

    async fn remove(&mut self, cid: Cid, recusrive: bool) -> Result<(), DataStoreError>;

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError>;

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        immediate: bool,
    ) -> Result<(), DataStoreError>;
}

#[async_trait(?Send)]
pub trait SyncableDataStore: DataStore + SyncTracker {
    async fn set_sync_host(&mut self, host: Url) -> Result<(), DataStoreError>;

    async fn store_sync(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), DataStoreError> {
        self.store(cid, data, true).await
    }

    async fn sync(&mut self, version_id: &str) -> Result<(), DataStoreError>;

    async fn unsynced_data_size(&self) -> Result<u64, DataStoreError> {
        self.tracked_size().await
    }
}

#[async_trait(?Send)]
pub trait SyncTracker {
    async fn clear_deleted(&mut self) -> Result<(), DataStoreError>;

    async fn delete(&mut self, cid: Cid) -> Result<(), DataStoreError>;

    async fn deleted_cids(&self) -> Result<Vec<Cid>, DataStoreError>;

    async fn track(&mut self, cid: Cid, size: u64) -> Result<(), DataStoreError>;

    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError>;

    async fn tracked_size(&self) -> Result<u64, DataStoreError>;

    async fn untrack(&mut self, cid: Cid) -> Result<(), DataStoreError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DataStoreError {
    #[error("failed to retrieve block")]
    LookupFailure,

    #[error("no storage hosts have been registered to interact with")]
    NoActiveStorageHost,

    #[error("failed to retreive block from network")]
    RetrievalFailure,

    #[error("failed to open storage session")]
    SessionRejected,

    #[error("failed to store block")]
    StoreFailure,

    #[error("block not available in this data store: {0:?}")]
    UnknownBlock(Cid),
}
