mod traits;

pub use traits::{DataStore, DataStoreError, SyncTracker, SyncableDataStore};

use std::collections::HashMap;
use std::sync::Arc;

use async_std::sync::RwLock;
use async_trait::async_trait;
use reqwest::Url;

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

    async fn remove(&mut self, cid: Cid, _recusrive: bool) -> Result<(), DataStoreError> {
        self.data.remove(&cid);
        Ok(())
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
        self.data.entry(cid).or_insert(data);

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
        self.tracked.entry(cid).or_insert(size);
        Ok(())
    }

    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        Ok(self.tracked.keys().cloned().collect())
    }

    async fn tracked_size(&self) -> Result<u64, DataStoreError> {
        Ok(self.tracked.values().sum())
    }

    async fn untrack(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.tracked.remove(&cid);
        Ok(())
    }
}

pub struct ApiSyncableStore<MS: DataStore, ST: SyncTracker> {
    client: ApiClient,
    inner: Arc<RwLock<ApiSyncableStoreInner<MS, ST>>>,
}

impl<MS: DataStore, ST: SyncTracker> ApiSyncableStore<MS, ST> {
    pub fn new(client: ApiClient, cached_store: MS, sync_tracker: ST) -> Self {
        let cid_map = HashMap::new();

        let inner = ApiSyncableStoreInner {
            cached_store,
            sync_tracker,
            cid_map,
        };

        Self {
            client,
            inner: Arc::new(RwLock::new(inner)),
        }
    }
}

#[async_trait(?Send)]
impl<MS: DataStore, ST: SyncTracker> DataStore for ApiSyncableStore<MS, ST> {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        self.inner
            .read()
            .await
            .contains_cid(&self.client, cid)
            .await
    }

    async fn remove(&mut self, cid: Cid, recursive: bool) -> Result<(), DataStoreError> {
        self.inner
            .write()
            .await
            .remove(&self.client, cid, recursive)
            .await
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        self.inner.read().await.retrieve(&self.client, cid).await
    }

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        immediate: bool,
    ) -> Result<(), DataStoreError> {
        self.inner
            .write()
            .await
            .store(&self.client, cid, data, immediate)
            .await
    }
}

#[async_trait(?Send)]
impl<MS: DataStore, ST: SyncTracker> SyncTracker for ApiSyncableStore<MS, ST> {
    async fn track(&mut self, cid: Cid, size: u64) -> Result<(), DataStoreError> {
        self.inner.write().await.sync_tracker.track(cid, size).await
    }

    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        self.inner.read().await.sync_tracker.tracked_cids().await
    }

    async fn tracked_size(&self) -> Result<u64, DataStoreError> {
        self.inner.read().await.sync_tracker.tracked_size().await
    }

    async fn untrack(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.inner.write().await.sync_tracker.untrack(cid).await
    }
}

#[async_trait(?Send)]
impl<MS: DataStore, ST: SyncTracker> SyncableDataStore for ApiSyncableStore<MS, ST> {
    async fn sync(&mut self) -> Result<(), DataStoreError> {
        self.inner.write().await.sync(&self.client).await
    }
}

impl<MS: DataStore, ST: SyncTracker> Clone for ApiSyncableStore<MS, ST> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            inner: self.inner.clone(),
        }
    }
}

pub struct ApiSyncableStoreInner<MS: DataStore, ST: SyncTracker> {
    cached_store: MS,
    sync_tracker: ST,

    // todo(sstelfox): need to expire this information
    cid_map: HashMap<Cid, Vec<Url>>,
}

impl<MS: DataStore, ST: SyncTracker> ApiSyncableStoreInner<MS, ST> {
    async fn contains_cid(&self, client: &ApiClient, cid: Cid) -> Result<bool, DataStoreError> {
        if self.cached_store.contains_cid(cid.clone()).await? {
            return Ok(true);
        }

        // todo(sstelfox): check cid map, do the dumb thing for now
        let locations = crate::api::platform::blocks::locate(client, &[cid.clone()])
            .await
            .map_err(|err| {
                tracing::error!("failed to locate block: {err}");
                DataStoreError::UnknownBlock(cid.clone())
            })?;

        if locations.is_missing(&cid) {
            tracing::error!("remote API doesn't know about the block: {cid:?}");
            return Err(DataStoreError::UnknownBlock(cid.clone()));
        }

        // todo(sstelfox): cache cid locations, for now we'll always do lookups which will be
        // slow...
        //let host_urls = match locations.storage_hosts_with_cid(cid)

        todo!("check blocks existence and location on the network")
    }

    async fn remove(
        &mut self,
        _client: &ApiClient,
        cid: Cid,
        recursive: bool,
    ) -> Result<(), DataStoreError> {
        self.cached_store.remove(cid.clone(), recursive).await?;
        self.sync_tracker.untrack(cid.clone()).await?;

        if recursive {
            todo!("delete from remote stores as well, remember return value");
        }

        Ok(())
    }

    async fn retrieve(&self, client: &ApiClient, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
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
        client: &ApiClient,
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

    async fn sync(&mut self, client: &ApiClient) -> Result<(), DataStoreError> {
        for cid in self.sync_tracker.tracked_cids().await? {
            let _data = self.cached_store.retrieve(cid.clone()).await?;

            // todo: push the block to the network

            self.sync_tracker.untrack(cid).await?;
        }

        Ok(())
    }
}
