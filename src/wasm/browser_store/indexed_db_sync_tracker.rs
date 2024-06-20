use async_trait::async_trait;
use web_sys::IdbDatabase;

use crate::codec::meta::Cid;
use crate::stores::{DataStoreError, SyncTracker};

const INDEXED_DB_NAME: &str = "banyanfs-sync-tracker";

pub struct IndexedDbSyncTracker {
    db: IdbDatabase,
}

impl IndexedDbSyncTracker {
    pub async fn new() -> Result<Self, IndexedDbSyncTrackerError> {
        let db = indexdb_handle().await?;
        Ok(Self { db })
    }
}

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

async fn indexdb_handle() -> Result<IdbDatabase, IndexedDbSyncTrackerError> {
    let window = match web_sys::window() {
        Some(window) => window,
        None => return Err(IndexedDbSyncTrackerError::WindowUnavailable),
    };

    let idb_factory = match window.indexed_db() {
        Ok(Some(factory)) => Ok(factory),
        Ok(None) => Err(IndexedDbSyncTrackerError::IndexedDbUnavailable),
        Err(err) => Err(IndexedDbSyncTrackerError::DbError(err.as_string())),
    }?;

    let open_request = match idb_factory.open(INDEXED_DB_NAME) {
        Ok(req) => req,
        Err(err) => return Err(IndexedDbSyncTrackerError::DbError(err.as_string())),
    };

    todo!();
}

#[derive(Debug, thiserror::Error)]
pub enum IndexedDbSyncTrackerError {
    #[error("error interacting with influxdb: {0:?}")]
    DbError(Option<String>),

    #[error("IndexedDB isn't available in the browser or has been disabled")]
    IndexedDbUnavailable,

    #[error("failed to get browser window object")]
    WindowUnavailable,
}
