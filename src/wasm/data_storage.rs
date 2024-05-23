use async_trait::async_trait;
//use js_sys::Uint8Array;
use tracing::warn;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemWritableFileStream, StorageManager,
};

use crate::api::ApiClient;
use crate::codec::meta::Cid;
use crate::stores::{
    ApiSyncableStore, DataStore, DataStoreError, MemoryDataStore, MemorySyncTracker,
};

// todo(sstelfox): I really want WASM aware versions of this that can be shared between browser
// tabs, likely this means using the filesystem for the data store and indexdb for the sync
// tracker.
pub(crate) type WasmDataStorage = ApiSyncableStore<MemoryDataStore, MemorySyncTracker>;

pub(crate) fn initialize_store(api_client: ApiClient) -> WasmDataStorage {
    let store = MemoryDataStore::default();
    let tracker = MemorySyncTracker::default();

    ApiSyncableStore::new(api_client, store, tracker)
}

//impl WasmDataStorage {
//    #[instrument(skip(self))]
//    pub async fn mark_synced(&self, cid: &Cid) -> Result<(), BanyanFsError> {
//        let mut inner = self.inner.write().await;
//        inner.mark_synced(cid).await
//    }
//
//    #[instrument(skip(self))]
//    pub async fn retrieve(&self, cid: &Cid) -> Result<Option<Vec<u8>>, BanyanFsError> {
//        let inner = self.inner.read().await;
//        inner.retrieve(cid).await
//
//        let raw_data = JsFuture::from(file.array_buffer())
//            .await
//            .map(Uint8Array::from)
//            .map(|a| a.to_vec())
//            .map_err(|e| format!("failed reading file data: {e:?}"))?;
//
//        Ok(Some(raw_data))
//    }
//
//    #[instrument(skip(self, data))]
//    pub async fn store(&self, cid: Cid, data: Vec<u8>) -> Result<(), BanyanFsError> {
//        let mut inner = self.inner.write().await;
//        inner.store(cid, data).await
//    }
//
//    pub async fn unsynced_cids(&self) -> Vec<Cid> {
//        let inner = self.inner.read().await;
//        inner.unsynced_cids()
//    }
//
//    pub async fn unsynced_data_size(&self) -> u64 {
//        let inner = self.inner.read().await;
//        inner.unsynced_data_size()
//    }
//}

//#[derive(Default)]
//struct DataStorageInner {
//    stored_cids: HashSet<Cid>,
//    unsynced_cids: HashSet<Cid>,
//    unsynced_data_size: u64,
//}

//impl DataStorageInner {
//    pub async fn retrieve(&self, cid: &Cid) -> Result<Option<Vec<u8>>, BanyanFsError> {
//        let file = match get_cid_file(cid).await? {
//            Some(file) => file,
//            None => return Ok(None),
//        };
//    }
//
//    pub async fn store(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), BanyanFsError> {
//        let storage_dir = storage_directory().await?;
//
//        let name = format!("{:?}.blk", cid.as_base64url_multicodec());
//        let mut open_opts = FileSystemGetFileOptions::new();
//        open_opts.create(true);
//
//        let fh = JsFuture::from(storage_dir.get_file_handle_with_options(&name, &open_opts))
//            .await
//            .map(FileSystemFileHandle::from)
//            .map_err(|e| format!("failed to open storage directory: {e:?}"))?;
//
//        let writer = JsFuture::from(fh.create_writable())
//            .await
//            .map(FileSystemWritableFileStream::from)
//            .map_err(|e| format!("failed to get writable file handle: {e:?}"))?;
//
//        let write_promise = writer
//            .write_with_u8_array(&data)
//            .map_err(|e| format!("failed to create storage future: {e:?}"))?;
//
//        JsFuture::from(write_promise)
//            .await
//            .map_err(|e| format!("failed to store data: {e:?}"))?;
//
//        self.stored_cids.insert(cid.clone());
//        self.unsynced_cids.insert(cid);
//        self.unsynced_data_size += data.len() as u64;
//
//        Ok(())
//    }
//
//    pub fn unsynced_cids(&self) -> Vec<Cid> {
//        self.unsynced_cids.iter().cloned().collect()
//    }
//
//    pub fn unsynced_data_size(&self) -> u64 {
//        self.unsynced_data_size
//    }
//}

//#[async_trait(?Send)]
//impl DataStore for DataStorage {
//    async fn retrieve(&self, cid: Cid) -> Result<Option<Vec<u8>>, DataStoreError> {
//        // todo(sstelfox): should attempt to retrieve from the storag network using the api client
//        // if not found locally
//        self.retrieve(&cid)
//            .await
//            .map_err(|_| DataStoreError::LookupFailure)
//    }
//
//    async fn store(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), DataStoreError> {
//        DataStorage::store(self, cid, data)
//            .await
//            .map_err(|_| DataStoreError::StoreFailure)
//    }
//}

//pub struct IndexedDbSyncTracker;

//#[async_trait(?Send)]
//impl SyncTracker for MemorySyncTracker {
//    todo!()
//}

pub struct WebFsDataStore;

#[async_trait(?Send)]
impl DataStore for WebFsDataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        let block = get_block(&cid).await?;
        Ok(block.is_some())
    }

    async fn remove(&mut self, cid: Cid, _recusrive: bool) -> Result<(), DataStoreError> {
        remove_block(&cid).await?;
        Ok(())
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        let _block_file = get_block(&cid).await?;

        todo!()
    }

    async fn store(
        &mut self,
        _cid: Cid,
        _data: Vec<u8>,
        _immediate: bool,
    ) -> Result<(), DataStoreError> {
        todo!()
    }
}

async fn get_block(cid: &Cid) -> Result<Option<File>, DataStoreError> {
    let storage_dir = storage_directory().await?;

    let name = format!("{:?}.blk", cid.as_base64url_multicodec());
    let fh = match JsFuture::from(storage_dir.get_file_handle(&name)).await {
        Ok(fh) => FileSystemFileHandle::from(fh),
        Err(err) => {
            warn!("error attempting to retrieve block {err:?}");
            return Ok(None);
        }
    };

    let file = match JsFuture::from(fh.get_file()).await {
        Ok(file) => File::from(file),
        Err(err) => {
            warn!("failed to retrieve file content: {err:?}");
            return Err(DataStoreError::RetrievalFailure);
        }
    };

    Ok(Some(file))
}

//async fn put_block(cid: &Cid, data: &[u8]) -> Result<(), DataStoreError> {
//    let storage_dir = storage_directory().await?;
//
//    let name = format!("{:?}.blk", cid.as_base64url_multicodec());
//    let mut open_opts = FileSystemGetFileOptions::new();
//    open_opts.create(true);
//
//    let fh = JsFuture::from(storage_dir.get_file_handle_with_options(&name, &open_opts))
//        .await
//        .map(FileSystemFileHandle::from)
//        .map_err(|e| format!("failed to open storage directory: {e:?}"))?;
//
//    let writer = JsFuture::from(fh.create_writable())
//        .await
//        .map(FileSystemWritableFileStream::from)
//        .map_err(|e| format!("failed to get writable file handle: {e:?}"))?;
//
//    let write_promise = writer
//        .write_with_u8_array(&data)
//        .map_err(|e| format!("failed to create storage future: {e:?}"))?;
//
//    JsFuture::from(write_promise)
//        .await
//        .map_err(|e| format!("failed to store data: {e:?}"))?;
//
//    Ok(())
//}

async fn remove_block(cid: &Cid) -> Result<(), WebFsDataStoreError> {
    let storage_dir = storage_directory().await?;

    let name = format!("{:?}.blk", cid.as_base64url_multicodec());
    if let Err(err) = JsFuture::from(storage_dir.remove_entry(&name)).await {
        let err_msg = format!("{err:?}");
        return Err(WebFsDataStoreError::RemovalFailed(err_msg))?;
    }

    Ok(())
}

async fn storage_directory() -> Result<FileSystemDirectoryHandle, WebFsDataStoreError> {
    let storage_manager = storage_manager().await?;

    let storage_dir = match JsFuture::from(storage_manager.get_directory()).await {
        Ok(dir) => FileSystemDirectoryHandle::from(dir),
        Err(err) => {
            let err_msg = format!("{err:?}");
            return Err(WebFsDataStoreError::StorageManagerUnavailable(err_msg));
        }
    };

    Ok(storage_dir)
}

async fn storage_manager() -> Result<StorageManager, WebFsDataStoreError> {
    let window = web_sys::window().ok_or(WebFsDataStoreError::WindowUnavailable)?;

    Ok(window.navigator().storage())
}

#[derive(Debug, thiserror::Error)]
pub enum WebFsDataStoreError {
    #[error("failed to remove block: {0}")]
    RemovalFailed(String),

    #[error("failed to retrieve storage manager")]
    StorageManagerUnavailable(String),

    #[error("failed to get browser window object")]
    WindowUnavailable,
}

impl From<WebFsDataStoreError> for DataStoreError {
    fn from(err: WebFsDataStoreError) -> DataStoreError {
        DataStoreError::Implementation(err.to_string())
    }
}
