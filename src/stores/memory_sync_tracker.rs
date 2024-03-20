use crate::stores::traits::{DataStoreError, SyncTracker};

use std::collections::HashMap;

use async_trait::async_trait;

use crate::codec::Cid;

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
