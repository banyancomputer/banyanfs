mod inner;

use inner::ApiSyncableStoreInner;

use std::sync::Arc;

use async_std::sync::RwLock;
use async_trait::async_trait;

use crate::api::ApiClient;
use crate::codec::Cid;
use crate::stores::{DataStore, DataStoreError, SyncTracker, SyncableDataStore};

pub struct ApiSyncableStore<MS: DataStore, ST: SyncTracker> {
    client: ApiClient,
    inner: Arc<RwLock<ApiSyncableStoreInner<MS, ST>>>,
}

impl<MS: DataStore, ST: SyncTracker> ApiSyncableStore<MS, ST> {
    pub fn new(client: ApiClient, cached_store: MS, sync_tracker: ST) -> Self {
        let inner = ApiSyncableStoreInner::new(cached_store, sync_tracker);

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
    async fn clear_deleted(&mut self) -> Result<(), DataStoreError> {
        self.inner
            .write()
            .await
            .sync_tracker_mut()
            .clear_deleted()
            .await
    }

    async fn delete(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.inner
            .write()
            .await
            .sync_tracker_mut()
            .delete(cid)
            .await
    }

    async fn deleted_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        self.inner.read().await.sync_tracker().deleted_cids().await
    }

    async fn track(&mut self, cid: Cid, size: u64) -> Result<(), DataStoreError> {
        self.inner
            .write()
            .await
            .sync_tracker_mut()
            .track(cid, size)
            .await
    }

    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        self.inner.read().await.sync_tracker().tracked_cids().await
    }

    async fn tracked_size(&self) -> Result<u64, DataStoreError> {
        self.inner.read().await.sync_tracker().tracked_size().await
    }

    async fn untrack(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.inner
            .write()
            .await
            .sync_tracker_mut()
            .untrack(cid)
            .await
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
