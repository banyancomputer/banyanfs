use std::collections::HashMap;

use async_trait::async_trait;

use crate::codec::Cid;
use crate::stores::traits::{DataStore, DataStoreError};

/// Simple minimal implementation of the [`DataStore`] trait. Simply stores all the provided blocks
/// in memory addressed by their CID. Currently used in our placeholder WASM implmentation before
/// blocks are synced to remote hosts (It's being combined with our [`crate::stores::ApiSyncableStore`]).
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
            .ok_or(DataStoreError::LookupFailure)
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
