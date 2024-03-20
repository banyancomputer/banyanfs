use crate::stores::traits::{DataStoreError, SyncTracker};

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use crate::codec::Cid;

#[derive(Default)]
pub struct MemorySyncTracker {
    pending_deletion: HashSet<Cid>,
    tracked: HashMap<Cid, u64>,
}

#[async_trait(?Send)]
impl SyncTracker for MemorySyncTracker {
    async fn clear_deleted(&mut self) -> Result<(), DataStoreError> {
        self.pending_deletion.clear();
        Ok(())
    }

    async fn delete(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.pending_deletion.insert(cid);
        Ok(())
    }

    async fn deleted_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        Ok(self.pending_deletion.iter().cloned().collect())
    }

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
