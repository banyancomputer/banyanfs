use async_trait::async_trait;
use js_sys::Uint8Array;
use tracing::warn;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    window, File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemWritableFileStream, StorageManager,
};

use crate::codec::meta::Cid;
use crate::stores::{DataStore, DataStoreError};
use crate::wasm::browser_store::worker;

pub struct WebFsDataStore {
    storage_dir: FileSystemDirectoryHandle,
}

impl WebFsDataStore {
    pub async fn new() -> Result<Self, WebFsDataStoreError> {
        let storage_manager = storage_manager().await?;
        let dir_handle_promise = storage_manager.get_directory();

        let storage_dir = match JsFuture::from(dir_handle_promise).await {
            Ok(dir) => FileSystemDirectoryHandle::from(dir),
            Err(err) => return Err(WebFsDataStoreError::DirectoryHandleError(err.as_string())),
        };

        Ok(Self { storage_dir })
    }
}

#[async_trait(?Send)]
impl DataStore for WebFsDataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        let name = block_fs_name(&cid);
        let handle_promise = self.storage_dir.get_file_handle(&name);

        match JsFuture::from(handle_promise).await {
            Ok(_) => Ok(true),
            Err(err) => {
                // todo: this returns a DomException, we need to return Ok(false) ONLY when the
                // error is "NotFoundError" but I need to trigger this error before I can see if I
                // can properly dyn cast it...
                warn!("CID existence check failed: {err:?}");
                Ok(false)
            }
        }
    }

    async fn remove(&mut self, cid: Cid, _recusrive: bool) -> Result<(), DataStoreError> {
        let name = block_fs_name(&cid);
        let removal_promise = self.storage_dir.remove_entry(&name);

        if let Err(err) = JsFuture::from(removal_promise).await {
            return Err(WebFsDataStoreError::RemovalFailed(err.as_string()))?;
        }

        Ok(())
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        //let block_file = match get_block(&cid).await? {
        //    Some(fh) => fh,
        //    None => return Err(DataStoreError::RetrievalFailure),
        //};

        //let arr_buf = match JsFuture::from(block_file.array_buffer()).await {
        //    Ok(ab) => ab,
        //    Err(err) => {
        //        let err_msg = format!("getting data from FS system: {err:?}");
        //        return Err(DataStoreError::Implementation(err_msg));
        //    }
        //};

        //let data = Uint8Array::from(arr_buf).to_vec();

        //Ok(data)
        todo!()
    }

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        _immediate: bool,
    ) -> Result<(), DataStoreError> {
        //if let Err(err) = put_block(&cid, &data).await {
        //    let err_msg = format!("failed to store block: {err:?}");
        //    return Err(DataStoreError::Implementation(err_msg));
        //}

        //Ok(())

        todo!()
    }
}

//async fn get_block(cid: &Cid) -> Result<Option<File>, DataStoreError> {
//    let storage_dir = storage_directory().await?;
//
//    let name = format!("{cid}.blk");
//    let fh = match JsFuture::from(storage_dir.get_file_handle(&name)).await {
//        Ok(fh) => FileSystemFileHandle::from(fh),
//        Err(err) => {
//            warn!("error attempting to retrieve block {err:?}");
//            return Ok(None);
//        }
//    };
//
//    let file = match JsFuture::from(fh.get_file()).await {
//        Ok(file) => File::from(file),
//        Err(err) => {
//            warn!("failed to retrieve file content: {err:?}");
//            return Err(DataStoreError::RetrievalFailure);
//        }
//    };
//
//    Ok(Some(file))
//}

//async fn put_block(cid: &Cid, data: &[u8]) -> Result<(), WebFsDataStoreError> {
//    let storage_dir = storage_directory().await?;
//    let name = format!("{cid}.blk");
//
//    let mut open_opts = FileSystemGetFileOptions::new();
//    open_opts.create(true);
//
//    let fh_promise = storage_dir.get_file_handle_with_options(&name, &open_opts);
//    let fh = match JsFuture::from(fh_promise).await {
//        Ok(fh) => FileSystemFileHandle::from(fh),
//        Err(err) => return Err(WebFsDataStoreError::FileHandleError(err.as_string())),
//    };
//
//    let writer = match JsFuture::from(fh.create_writable()).await {
//        Ok(writer) => FileSystemWritableFileStream::from(writer),
//        Err(err) => {
//            let err_msg = format!("failed to make file handle writable: {err:?}");
//            return Err(WebFsDataStoreError::FileHandleError(Some(err_msg)));
//        }
//    };
//
//    let write_promise = match writer.write_with_u8_array(&data) {
//        Ok(promise) => promise,
//        Err(err) => {
//            let err_msg = format!("failed to create storage future: {err:?}");
//            return Err(WebFsDataStoreError::PromiseError(err_msg));
//        }
//    };
//
//    if let Err(err) = JsFuture::from(write_promise).await {
//        let err_msg = format!("write failed: {err:?}");
//        return Err(WebFsDataStoreError::FileHandleError(Some(err_msg)));
//    }
//
//    Ok(())
//}

fn block_fs_name(cid: &Cid) -> String {
    format!("{cid}.blk")
}

async fn storage_manager() -> Result<StorageManager, WebFsDataStoreError> {
    if let Some(sm) = window_storage_manager().await? {
        return Ok(sm);
    }

    if let Some(sm) = worker_storage_manager().await? {
        return Ok(sm);
    }

    Err(WebFsDataStoreError::StorageManagerUnavailable)
}

async fn window_storage_manager() -> Result<Option<StorageManager>, WebFsDataStoreError> {
    let window = match window() {
        Some(window) => window,
        None => return Ok(None),
    };

    Ok(Some(window.navigator().storage()))
}

async fn worker_storage_manager() -> Result<Option<StorageManager>, WebFsDataStoreError> {
    let worker = match worker() {
        Some(worker) => worker,
        None => return Ok(None),
    };

    Ok(Some(worker.navigator().storage()))
}

#[derive(Debug, thiserror::Error)]
pub enum WebFsDataStoreError {
    #[error("failed to get hold on directory handle: {0:?}")]
    DirectoryHandleError(Option<String>),

    #[error("failed to get hold on file handle: {0:?}")]
    FileHandleError(Option<String>),

    #[error("failed to generate a JS promise: {0}")]
    PromiseError(String),

    #[error("failed to remove block: {0:?}")]
    RemovalFailed(Option<String>),

    #[error("failed to retrieve storage manager")]
    StorageManagerUnavailable,
}

impl From<WebFsDataStoreError> for DataStoreError {
    fn from(err: WebFsDataStoreError) -> DataStoreError {
        DataStoreError::Implementation(err.to_string())
    }
}
