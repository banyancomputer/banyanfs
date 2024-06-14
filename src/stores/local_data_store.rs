use async_trait::async_trait;
use directories::ProjectDirs;
use std::path::PathBuf;

use crate::codec::Cid;
use crate::stores::traits::{DataStore, DataStoreError};

pub struct LocalDataStore {
    data_dir: PathBuf,
}

impl LocalDataStore {
    fn cid_to_path(&self, cid: &Cid) -> PathBuf {
        self.data_dir.join(cid.to_string())
    }
}

#[async_trait(?Send)]
impl DataStore for LocalDataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        Ok(self.cid_to_path(&cid).exists())
    }

    async fn remove(&mut self, cid: Cid, _recursive: bool) -> Result<(), DataStoreError> {
        let path = self.cid_to_path(&cid);
        if path.exists() {
            std::fs::remove_file(path).map_err(|_| DataStoreError::StoreFailure)?;
        }
        Ok(())
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        let path = self.cid_to_path(&cid);
        std::fs::read(path).map_err(|_| DataStoreError::LookupFailure)
    }

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        _immediate: bool,
    ) -> Result<(), DataStoreError> {
        let path = self.cid_to_path(&cid);
        std::fs::write(path, data).map_err(|_| DataStoreError::StoreFailure)?;
        Ok(())
    }
}

impl Default for LocalDataStore {
    fn default() -> Self {
        let proj_dirs = ProjectDirs::from("computer", "Banyan", "banyan-fuse").unwrap();
        let data_dir = proj_dirs.data_dir().to_owned();
        std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
        LocalDataStore { data_dir }
    }
}
