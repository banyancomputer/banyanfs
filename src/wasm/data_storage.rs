use std::collections::HashSet;

use js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemWritableFileStream, StorageManager,
};

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
    pub async fn mark_synced(&mut self, cid: Cid) -> Result<(), BanyanFsError> {
        if self.unsynced_cids.contains(&cid) {
            self.unsynced_cids.remove(&cid);
            self.unsynced_data_size -= size_of_cid_file(&cid).await?;

            // For now we're just going to remove the local storage as well once we've synced it,
            // this is a place where we can get some easy in browser performance wins by re-using
            // this as a block cache but that involves manual memory management beyond the scope of
            // this MVP.
            remove_cid_file(&cid).await?;
            self.stored_cids.remove(&cid);
        }

        Ok(())
    }

    pub async fn retrieve(&self, cid: Cid) -> Result<Option<Vec<u8>>, BanyanFsError> {
        let file = match get_cid_file(&cid).await? {
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

    pub fn unsynced_data_size(&self) -> u64 {
        self.unsynced_data_size
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
