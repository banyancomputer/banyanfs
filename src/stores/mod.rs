mod traits;

pub use traits::{DataStore, DataStoreError, SyncTracker, SyncableDataStore};

use std::collections::HashMap;

use async_trait::async_trait;

use crate::api::ApiClient;
use crate::codec::Cid;

#[derive(Default)]
pub struct MemoryDataStore {
    data: HashMap<Cid, Vec<u8>>,
}

#[async_trait(?Send)]
impl DataStore for MemoryDataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        Ok(self.data.contains_key(&cid))
    }

    async fn remove(&mut self, cid: Cid, _recusrive: bool) -> Result<Option<u64>, DataStoreError> {
        let removed_data = self.data.remove(&cid).map(|d| d.len() as u64);
        Ok(removed_data)
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        self.data
            .get(&cid)
            .cloned()
            .ok_or(DataStoreError::UnknownBlock(cid))
    }

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        _immediate: bool,
    ) -> Result<(), DataStoreError> {
        // We assume that CIDs are universally unique, if we're already storing a CID don't shuffle
        // our memory around again for a new one.
        if !self.data.contains_key(&cid) {
            self.data.insert(cid, data);
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct MemorySyncTracker {
    tracked: HashMap<Cid, u64>,
}

#[async_trait(?Send)]
impl SyncTracker for MemorySyncTracker {
    async fn track(&mut self, cid: Cid, size: u64) -> Result<(), DataStoreError> {
        todo!()
    }

    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        todo!()
    }

    async fn tracked_size(&self) -> Result<u64, DataStoreError> {
        todo!()
    }

    async fn untrack(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        todo!()
    }
}

pub struct ApiSyncableStore<MS: DataStore, ST: SyncTracker> {
    client: ApiClient,

    cached_store: MS,
    sync_tracker: ST,
}

impl<MS: DataStore, ST: SyncTracker> ApiSyncableStore<MS, ST> {
    pub fn new(client: ApiClient, cached_store: MS, sync_tracker: ST) -> Self {
        Self {
            client,

            cached_store,
            sync_tracker,
        }
    }
}

#[async_trait(?Send)]
impl<MS: DataStore, ST: SyncTracker> DataStore for ApiSyncableStore<MS, ST> {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        if self.cached_store.contains_cid(cid.clone()).await? {
            return Ok(true);
        }

        todo!("check blocks existence and location on the network")
    }

    async fn remove(&mut self, cid: Cid, recursive: bool) -> Result<Option<u64>, DataStoreError> {
        if let Some(data_length) = self.cached_store.remove(cid.clone(), recursive).await? {
            self.sync_tracker.untrack(cid.clone()).await?;
        }

        if recursive {
            todo!("delete from remote stores as well, remember return value");
        }

        Ok(None)
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        if self.cached_store.contains_cid(cid.clone()).await? {
            return self.cached_store.retrieve(cid).await;
        }

        // check whether the block is available on the network and if so where
        // retrieve block from the network and cache it
        // return the block
        todo!("fall back to network retrieval and cache the block")
    }

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        immediate: bool,
    ) -> Result<(), DataStoreError> {
        if !self.cached_store.contains_cid(cid.clone()).await? {
            let data_length = data.len() as u64;

            self.cached_store
                .store(cid.clone(), data, immediate)
                .await?;
            self.sync_tracker.track(cid.clone(), data_length).await?;
        }

        if immediate {
            // todo: push the block to the network
            // todo: mark the block as synced
        }

        Ok(())
    }
}

#[async_trait(?Send)]
impl<MS: DataStore, ST: SyncTracker> SyncableDataStore for ApiSyncableStore<MS, ST> {
    type Tracker = ST;

    async fn sync(&mut self) -> Result<(), DataStoreError> {
        todo!()
    }

    fn tracker(&self) -> &ST {
        &self.sync_tracker
    }

    fn tracker_mut(&mut self) -> &mut ST {
        &mut self.sync_tracker
    }
}
