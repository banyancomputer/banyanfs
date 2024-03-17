use std::collections::HashSet;

use crate::codec::Cid;
use crate::error::BanyanFsError;

#[derive(Default)]
pub struct DataStorage {
    // note(sstelfox): these values should live in indexdb so multiple browser windows can access
    // the same information...
    stored_cids: HashSet<Cid>,
    unsynced_cids: HashSet<Cid>,
    unsynced_data_size: u64,
}

impl DataStorage {
    pub async fn retrieve(&self, _cid: Cid) -> Result<Option<Vec<u8>>, BanyanFsError> {
        todo!()
    }

    pub async fn store(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), BanyanFsError> {
        use wasm_bindgen_futures::JsFuture;

        let window = web_sys::window().ok_or("failed to get browser window object")?;
        let storage_manager = window.navigator().storage();
        let storage_dir = JsFuture::from(storage_manager.get_directory())
            .await
            .map_err(|e| format!("failed to resolve storage manager: {e:?}"))?;

        self.stored_cids.insert(cid.clone());
        self.unsynced_cids.insert(cid);
        self.unsynced_data_size += data.len() as u64;

        Ok(())
    }

    pub async fn mark_synced(&mut self, cid: Cid) -> Result<(), BanyanFsError> {
        if self.unsynced_cids.contains(&cid) {
            self.unsynced_cids.remove(&cid);
            // todo: reduce self.unsynced_data_size by size of stored data
        }

        Ok(())
    }

    pub fn unsynced_data_size(&self) -> u64 {
        self.unsynced_data_size
    }
}
