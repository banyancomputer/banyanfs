use std::collections::HashMap;

use async_trait::async_trait;

use crate::codec::Cid;
use crate::filesystem::{DataStore, DataStoreError};

#[derive(Default)]
pub struct MemoryStore {
    data: HashMap<Cid, Vec<u8>>,
}

#[async_trait(?Send)]
impl DataStore for MemoryStore {
    async fn retrieve(&self, cid: Cid) -> Result<Option<Vec<u8>>, DataStoreError> {
        let data = self.data.get(&cid).cloned();
        Ok(data)
    }

    async fn store(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), DataStoreError> {
        self.data.insert(cid, data);
        Ok(())
    }
}
