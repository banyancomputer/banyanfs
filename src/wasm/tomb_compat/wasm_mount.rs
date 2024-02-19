use async_std::sync::{Arc, RwLock};

use crate::prelude::*;

use js_sys::{Array, ArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::filesystem::Drive;
use crate::wasm::tomb_compat::{TombCompat, WasmBucket, WasmBucketMetadata, WasmSnapshot};

#[derive(Clone)]
#[wasm_bindgen]
pub struct WasmMount {
    wasm_client: TombCompat,

    bucket_id: String,
    drive: Option<Arc<RwLock<Drive>>>,
}

impl WasmMount {
    pub(crate) fn new(bucket_id: String, wasm_client: TombCompat) -> Self {
        Self {
            wasm_client,

            bucket_id,
            drive: None,
        }
    }
}

#[wasm_bindgen]
impl WasmMount {
    // appears deprecated
    //pub async fn add(
    //    &mut self,
    //    _path_segments: js_sys::Array,
    //    _content_buffer: js_sys::ArrayBuffer,
    //) -> BanyanFsResult<()> {
    //    todo!()
    //}

    // new, checked
    pub fn bucket(&self) -> WasmBucket {
        todo!()
    }

    // checked
    pub fn dirty(&self) -> bool {
        todo!()
    }

    // checked
    #[wasm_bindgen(js_name = hasSnapshot)]
    pub fn has_snapshot(&self) -> bool {
        tracing::warn!("not reporting snapshots as it hasn't been implemented yet");
        false
    }

    // checked
    pub fn locked(&self) -> bool {
        self.drive.is_none()
    }

    // checked, returns list of WasmFsMetadataEntry instances
    pub async fn ls(&mut self, _path_segments: js_sys::Array) -> BanyanFsResult<js_sys::Array> {
        todo!()
    }

    // checked
    pub async fn metadata(&self) -> BanyanFsResult<WasmBucketMetadata> {
        todo!()
    }

    // checked
    pub async fn mkdir(&mut self, _path_segments: js_sys::Array) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    pub async fn mv(
        &mut self,
        _from_path_segments: js_sys::Array,
        _to_path_segments: js_sys::Array,
    ) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    #[wasm_bindgen(js_name = readBytes)]
    pub async fn read_bytes(
        &mut self,
        _path_segments: js_sys::Array,
        _version: Option<String>,
    ) -> BanyanFsResult<Uint8Array> {
        todo!()
    }

    // checked
    #[wasm_bindgen]
    pub async fn remount(&mut self, _key_pem: String) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    pub async fn rename(&mut self, _name: String) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    pub async fn restore(&mut self, _wasm_snapshot: WasmSnapshot) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    pub async fn rm(&mut self, _path_segments: js_sys::Array) -> BanyanFsResult<()> {
        todo!()
    }

    // checked, returns URL to access file
    #[wasm_bindgen(js_name = shareFile)]
    pub async fn share_file(&mut self, _path_segments: js_sys::Array) -> BanyanFsResult<String> {
        todo!()
    }

    // checked
    #[wasm_bindgen(js_name = shareWith)]
    pub async fn share_with(&mut self, _bucket_key_id: String) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    #[wasm_bindgen(js_name = snapshot)]
    pub async fn snapshot(&mut self) -> BanyanFsResult<String> {
        todo!()
    }

    // checked
    pub async fn write(
        &mut self,
        _path_segments: Array,
        _content_buffer: ArrayBuffer,
    ) -> BanyanFsResult<()> {
        todo!()
    }
}
