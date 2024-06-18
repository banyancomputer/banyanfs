pub use crate::stores::traits::{DataStore, DataStoreError, SyncTracker};

use std::collections::HashMap;

use reqwest::Url;

use crate::api::ApiClient;
use crate::codec::Cid;

pub struct ApiSyncableStoreInner<MS: DataStore, ST: SyncTracker> {
    cached_store: MS,
    sync_tracker: ST,

    sync_host: Option<Url>,

    // todo(sstelfox): need to expire this information
    cid_map: HashMap<Cid, Vec<Url>>,
}

impl<MS: DataStore, ST: SyncTracker> ApiSyncableStoreInner<MS, ST> {
    pub(crate) async fn contains_cid(
        &mut self,
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
                DataStoreError::LookupFailure
            })?;

        if locations.is_missing(&cid) {
            tracing::error!("remote API doesn't know about the block: {cid:?}");
            return Err(DataStoreError::LookupFailure);
        }

        if let Some(locs) = locations.storage_hosts_with_cid(&cid) {
            self.cid_map.insert(cid.clone(), locs);
        }

        Ok(locations.contains_cid(&cid))
    }

    pub(crate) fn new(cached_store: MS, sync_tracker: ST) -> Self {
        Self {
            cached_store,
            sync_tracker,
            sync_host: None,
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
        &mut self,
        client: &ApiClient,
        cid: Cid,
    ) -> Result<Vec<u8>, DataStoreError> {
        use crate::api::platform::blocks as platform_blocks;
        use crate::api::storage_host::blocks as storage_blocks;

        tracing::info!("retrieving block: {cid:?}");

        if self.cached_store.contains_cid(cid.clone()).await? {
            return self.cached_store.retrieve(cid).await;
        }

        // If we don't locally know about the block check the network store to see if it knows
        // about it. This also populates the cid_map with the location so we can immediately use
        // it.
        let mut block_hosts = match self.cid_map.get(&cid) {
            Some(hosts) => hosts.clone(),
            None => {
                let locations = platform_blocks::locate(client, &[cid.clone()])
                    .await
                    .map_err(|err| {
                        tracing::error!("failed to locate block: {err}");
                        DataStoreError::LookupFailure
                    })?;

                if locations.is_missing(&cid) {
                    tracing::error!("remote API doesn't know about the block: {cid:?}");
                    return Err(DataStoreError::LookupFailure);
                }

                let hosts = locations.storage_hosts_with_cid(&cid).ok_or_else(|| {
                    tracing::error!("no storage hosts known for block: {cid:?}");
                    DataStoreError::LookupFailure
                })?;

                self.cid_map.insert(cid.clone(), hosts.clone());

                hosts
            }
        };

        use rand::seq::SliceRandom;
        let mut rng = crate::utils::crypto_rng();
        block_hosts.shuffle(&mut rng);

        use crate::api::client::utils::consume_stream_into_bytes;

        for host in block_hosts.iter().take(3) {
            let cid_str = cid.as_base64url_multicodec();

            let block = match storage_blocks::retrieve(client, host, &cid_str).await {
                Ok(block) => block,
                Err(err) => {
                    tracing::error!("failed to retrieve block from {host}: {err}");
                    continue;
                }
            };

            let block_data = consume_stream_into_bytes(block)
                .await
                .map(|data| data.to_vec())
                .map_err(|err| {
                    tracing::error!("failed to consume block stream: {err}");
                    DataStoreError::RetrievalFailure
                })?;

            self.cached_store
                .store(cid.clone(), block_data.clone(), false)
                .await?;

            return Ok(block_data);
        }

        Err(DataStoreError::RetrievalFailure)
    }

    pub(crate) async fn set_sync_host(&mut self, host: Url) -> Result<(), DataStoreError> {
        self.sync_host = Some(host);
        Ok(())
    }

    pub(crate) async fn store(
        &mut self,
        _client: &ApiClient,
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
            tracing::warn!("immediate storage of blocks isn't supported, only bulk metadata tagged uploads for now");
        }

        Ok(())
    }

    pub(crate) async fn sync(
        &mut self,
        client: &ApiClient,
        metadata_id: &str,
    ) -> Result<(), DataStoreError> {
        use crate::api::storage_host::blocks;

        let tracked_cids = self.sync_tracker.tracked_cids().await?;
        if tracked_cids.is_empty() {
            // No sync necessary
            return Ok(());
        }

        let storage_host_url = self
            .sync_host
            .clone()
            .ok_or(DataStoreError::NoActiveStorageHost)?;

        let session_data_size = self.sync_tracker.tracked_size().await?;

        let session =
            blocks::create_session(client, &storage_host_url, metadata_id, session_data_size)
                .await
                .map_err(|_| DataStoreError::SessionRejected)?;

        let upload_id = session.upload_id();
        let cid_count = tracked_cids.len();

        for (idx, cid) in tracked_cids.into_iter().enumerate() {
            let data = self.cached_store.retrieve(cid.clone()).await?;
            let block_stream = crate::api::client::utils::VecStream::new(data).pinned();

            tracing::info!(?cid, "syncing block to the network");

            if idx == cid_count - 1 {
                // If we're the last one, we need to tweak our request
                blocks::store_complete(client, &storage_host_url, upload_id, &cid, block_stream)
                    .await
                    .map_err(|_| DataStoreError::StoreFailure)?
            } else {
                blocks::store_ongoing(client, &storage_host_url, upload_id, &cid, block_stream)
                    .await
                    .map_err(|_| DataStoreError::StoreFailure)?
            }

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
