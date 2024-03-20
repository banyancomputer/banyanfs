pub use crate::stores::traits::{DataStore, DataStoreError, SyncTracker};

use std::collections::HashMap;

use reqwest::Url;

use crate::api::ApiClient;
use crate::codec::Cid;

pub struct ApiSyncableStoreInner<MS: DataStore, ST: SyncTracker> {
    cached_store: MS,
    sync_tracker: ST,

    // todo(sstelfox): need to expire this information
    cid_map: HashMap<Cid, Vec<Url>>,
}

impl<MS: DataStore, ST: SyncTracker> ApiSyncableStoreInner<MS, ST> {
    pub(crate) async fn contains_cid(
        &self,
        client: &ApiClient,
        cid: Cid,
    ) -> Result<bool, DataStoreError> {
        if self.cached_store.contains_cid(cid.clone()).await? {
            return Ok(true);
        }

        if self.cid_map.contains_key(&cid) {
            return Ok(true);
        }

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

        Ok(locations.contains_cid(&cid))
    }

    pub(crate) fn new(cached_store: MS, sync_tracker: ST) -> Self {
        Self {
            cached_store,
            sync_tracker,
            cid_map: HashMap::new(),
        }
    }

    pub(crate) async fn remove(
        &mut self,
        _client: &ApiClient,
        cid: Cid,
        recursive: bool,
    ) -> Result<(), DataStoreError> {
        self.cached_store.remove(cid.clone(), recursive).await?;
        self.sync_tracker.untrack(cid.clone()).await?;

        if recursive {
            self.sync_tracker.delete(cid).await?;
        }

        Ok(())
    }

    pub(crate) async fn retrieve(
        &self,
        client: &ApiClient,
        cid: Cid,
    ) -> Result<Vec<u8>, DataStoreError> {
        if self.cached_store.contains_cid(cid.clone()).await? {
            return self.cached_store.retrieve(cid).await;
        }

        // check whether the block is available on the network and if so where
        // retrieve block from the network and cache it
        // return the block
        todo!("fall back to network retrieval and cache the block")
    }

    pub(crate) async fn store(
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

    pub(crate) async fn sync(&mut self, client: &ApiClient) -> Result<(), DataStoreError> {
        for cid in self.sync_tracker.tracked_cids().await? {
            let _data = self.cached_store.retrieve(cid.clone()).await?;

            // todo: push the block to the network

            self.sync_tracker.untrack(cid).await?;
        }

        Ok(())
    }

    pub(crate) fn sync_tracker(&self) -> &ST {
        &self.sync_tracker
    }

    pub(crate) fn sync_tracker_mut(&mut self) -> &mut ST {
        &mut self.sync_tracker
    }
}
