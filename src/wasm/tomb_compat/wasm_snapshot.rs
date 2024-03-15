use wasm_bindgen::prelude::*;

use crate::api::platform::{ApiDriveId, ApiSnapshot};

#[wasm_bindgen]
pub struct WasmSnapshot(ApiDriveId, ApiSnapshot);

impl WasmSnapshot {
    pub(crate) fn new(bucket_id: ApiDriveId, snapshot: ApiSnapshot) -> Self {
        Self(bucket_id, snapshot)
    }
}

#[wasm_bindgen]
impl WasmSnapshot {
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        self.0.clone()
    }

    // note(sstelfox): This was returning a string... but it looks like it might've just been
    // stringifying the number...
    #[wasm_bindgen(getter = createdAt)]
    pub fn created_at(&self) -> i64 {
        self.1.created_at()
    }

    pub fn id(&self) -> String {
        self.1.id()
    }

    #[wasm_bindgen(js_name = metadataId)]
    pub fn metadata_id(&self) -> String {
        self.1.metadata_id()
    }

    // note(sstelfox): We may not know the size of the snapshot, this should have some concept of
    // optional instead of 0
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> i64 {
        self.1.size().unwrap_or(0)
    }
}
