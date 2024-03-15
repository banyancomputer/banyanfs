use wasm_bindgen::prelude::*;

use crate::api::platform::{ApiDriveId, ApiMetadata};

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmBucketMetadata(ApiDriveId, ApiMetadata);

impl WasmBucketMetadata {
    pub(crate) fn new(bucket_id: String, metadata: ApiMetadata) -> Self {
        WasmBucketMetadata(bucket_id.into(), metadata)
    }

    pub(crate) fn api_metadata(&self) -> &ApiMetadata {
        &self.1
    }
}

#[wasm_bindgen]
impl WasmBucketMetadata {
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        self.0.clone()
    }

    // note(sstelfox): this is a metadata ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.1.id()
    }

    // note(sstelfox): this is wrong, snapshot's aren't guaranteed to be present, this needs t be
    // an Option, for now we'll return an empty string when not present
    #[wasm_bindgen(getter = snapshotId)]
    pub fn snapshot_id(&self) -> String {
        self.1.snapshot_id().unwrap_or_default()
    }
}
