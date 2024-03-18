use std::collections::HashMap;

use async_trait::async_trait;

use crate::codec::Cid;
use crate::filesystem::{DataStore, DataStoreError};

#[derive(Default)]
pub struct MemoryStore {
    data: HashMap<Cid, Vec<u8>>,
    unsynced_data_size: u64,
}

#[async_trait(?Send)]
impl DataStore for MemoryStore {
    type Client = ();

    async fn retrieve(
        &self,
        _client: &Self::Client,
        cid: Cid,
    ) -> Result<Option<Vec<u8>>, DataStoreError> {
        Ok(self.data.get(&cid).cloned())
    }

    async fn store(
        &mut self,
        _client: &Self::Client,
        cid: Cid,
        data: Vec<u8>,
    ) -> Result<(), DataStoreError> {
        self.data.insert(cid, data);
        Ok(())
    }

    async fn sync(&mut self, _client: &Self::Client) -> Result<(), DataStoreError> {
        self.unsynced_data_size = 0;

        Ok(())
    }

    async fn unsynced_data_size(&self) -> u64 {
        self.unsynced_data_size
    }
}
