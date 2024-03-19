use std::collections::HashSet;
use std::sync::Arc;

use async_std::sync::RwLock;
use async_trait::async_trait;
use js_sys::Uint8Array;
use tracing::instrument;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemWritableFileStream, StorageManager,
};

use crate::api::{storage_host, ApiClient};
use crate::codec::Cid;
use crate::error::BanyanFsError;
use crate::filesystem::{DataStore, DataStoreError, DelayedDataStore};

#[derive(Clone, Default)]
pub struct DataStorage {
    // note(sstelfox): these values should live in indexdb so multiple browser windows can access
    // the same information...
    inner: Arc<RwLock<DataStorageInner>>,
}

impl DataStorage {
    #[instrument(skip(self))]
    pub async fn mark_synced(&self, cid: &Cid) -> Result<(), BanyanFsError> {
        let mut inner = self.inner.write().await;
        inner.mark_synced(cid).await
    }

    #[instrument(skip(self))]
    pub async fn retrieve(&self, cid: &Cid) -> Result<Option<Vec<u8>>, BanyanFsError> {
        let inner = self.inner.read().await;
        inner.retrieve(cid).await
    }

    #[instrument(skip(self, data))]
    pub async fn store(&self, cid: Cid, data: Vec<u8>) -> Result<(), BanyanFsError> {
        let mut inner = self.inner.write().await;
        inner.store(cid, data).await
    }

    pub async fn unsynced_cids(&self) -> Vec<Cid> {
        let inner = self.inner.read().await;
        inner.unsynced_cids()
    }

    pub async fn unsynced_data_size(&self) -> u64 {
        let inner = self.inner.read().await;
        inner.unsynced_data_size()
    }
}

#[derive(Default)]
struct DataStorageInner {
    stored_cids: HashSet<Cid>,
    unsynced_cids: HashSet<Cid>,
    unsynced_data_size: u64,
}

impl DataStorageInner {
    pub async fn mark_synced(&mut self, cid: &Cid) -> Result<(), BanyanFsError> {
        if self.unsynced_cids.contains(cid) {
            self.unsynced_cids.remove(cid);
            self.unsynced_data_size -= size_of_cid_file(cid).await?;

            // For now we're just going to remove the local storage as well once we've synced it,
            // this is a place where we can get some easy in browser performance wins by re-using
            // this as a block cache but that involves manual memory management beyond the scope of
            // this MVP.
            //
            // We're not going to do this until we're actually writing the synced data remotely
            //
            //remove_cid_file(cid).await?;
            //self.stored_cids.remove(cid);
        }

        Ok(())
    }

    pub async fn retrieve(&self, cid: &Cid) -> Result<Option<Vec<u8>>, BanyanFsError> {
        let file = match get_cid_file(cid).await? {
            Some(file) => file,
            None => return Ok(None),
        };

        let raw_data = JsFuture::from(file.array_buffer())
            .await
            .map(Uint8Array::from)
            .map(|a| a.to_vec())
            .map_err(|e| format!("failed reading file data: {e:?}"))?;

        Ok(Some(raw_data))
    }

    pub async fn store(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), BanyanFsError> {
        let storage_dir = storage_directory().await?;

        let name = format!("{:?}.blk", cid.as_base64url_multicodec());
        let mut open_opts = FileSystemGetFileOptions::new();
        open_opts.create(true);

        let fh = JsFuture::from(storage_dir.get_file_handle_with_options(&name, &open_opts))
            .await
            .map(FileSystemFileHandle::from)
            .map_err(|e| format!("failed to open storage directory: {e:?}"))?;

        let writer = JsFuture::from(fh.create_writable())
            .await
            .map(FileSystemWritableFileStream::from)
            .map_err(|e| format!("failed to get writable file handle: {e:?}"))?;

        let write_promise = writer
            .write_with_u8_array(&data)
            .map_err(|e| format!("failed to create storage future: {e:?}"))?;

        JsFuture::from(write_promise)
            .await
            .map_err(|e| format!("failed to store data: {e:?}"))?;

        self.stored_cids.insert(cid.clone());
        self.unsynced_cids.insert(cid);
        self.unsynced_data_size += data.len() as u64;

        Ok(())
    }

    pub fn unsynced_cids(&self) -> Vec<Cid> {
        self.unsynced_cids.iter().cloned().collect()
    }

    pub fn unsynced_data_size(&self) -> u64 {
        self.unsynced_data_size
    }
}

#[async_trait(?Send)]
impl DataStore for DataStorage {
    async fn retrieve(&self, cid: Cid) -> Result<Option<Vec<u8>>, DataStoreError> {
        // todo(sstelfox): should attempt to retrieve from the storag network using the api client
        // if not found locally
        self.retrieve(&cid)
            .await
            .map_err(|_| DataStoreError::LookupFailure)
    }

    async fn store(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), DataStoreError> {
        DataStorage::store(self, cid, data)
            .await
            .map_err(|_| DataStoreError::StoreFailure)
    }
}

#[async_trait(?Send)]
impl DelayedDataStore for DataStorage {
    type Client = ApiClient;

    async fn sync(&mut self, client: &Self::Client) -> Result<(), DataStoreError> {
        let to_sync = self.unsynced_cids().await;
        let mut inner_write = self.inner.write().await;

        for cid in to_sync.iter() {
            let data = match inner_write
                .retrieve(cid)
                .await
                .map_err(|_| DataStoreError::LookupFailure)?
            {
                Some(data) => data,
                None => {
                    tracing::warn!("didn't have copy of block requiring sync: {cid:?}, unsynced data size may be out of sync");

                    inner_write.unsynced_cids.remove(cid);
                    inner_write.stored_cids.remove(cid);

                    continue;
                }
            };

            storage_host::blocks::store(client, cid, &data)
                .await
                .map_err(|_| DataStoreError::StoreFailure)?;

            inner_write
                .mark_synced(cid)
                .await
                .map_err(|_| DataStoreError::StoreFailure)?;
        }

        inner_write.unsynced_data_size = 0;

        Ok(())
    }

    async fn unsynced_data_size(&self) -> u64 {
        let inner = self.inner.read().await;
        inner.unsynced_data_size()
    }
}

async fn get_cid_file(cid: &Cid) -> Result<Option<File>, BanyanFsError> {
    let storage_dir = storage_directory().await?;

    let name = format!("{:?}.blk", cid.as_base64url_multicodec());
    let fh = match JsFuture::from(storage_dir.get_file_handle(&name)).await {
        Ok(fh) => FileSystemFileHandle::from(fh),
        Err(_) => return Ok(None),
    };

    let file = JsFuture::from(fh.get_file())
        .await
        .map(File::from)
        .map_err(|e| format!("failed to retrieve file content: {e:?}"))?;

    Ok(Some(file))
}

async fn remove_cid_file(cid: &Cid) -> Result<(), BanyanFsError> {
    let storage_dir = storage_directory().await?;

    let name = format!("{:?}.blk", cid.as_base64url_multicodec());
    JsFuture::from(storage_dir.remove_entry(&name))
        .await
        .map_err(|e| format!("failed to remove file: {e:?}"))?;

    Ok(())
}

async fn size_of_cid_file(cid: &Cid) -> Result<u64, BanyanFsError> {
    let _file = match get_cid_file(cid).await? {
        Some(file) => file,
        None => return Ok(0),
    };

    todo!()
}

async fn storage_directory() -> Result<FileSystemDirectoryHandle, BanyanFsError> {
    let storage_manager = storage_manager().await?;

    let storage_dir: FileSystemDirectoryHandle = JsFuture::from(storage_manager.get_directory())
        .await
        .map_err(|e| format!("failed to resolve storage manager: {e:?}"))?
        .into();

    Ok(storage_dir)
}

async fn storage_manager() -> Result<StorageManager, BanyanFsError> {
    let window = web_sys::window().ok_or("failed to get browser window object")?;
    Ok(window.navigator().storage())
}
