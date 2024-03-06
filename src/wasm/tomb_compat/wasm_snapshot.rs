use wasm_bindgen::prelude::*;

use crate::api::platform::{ApiDriveId, ApiSnapshot};

#[wasm_bindgen]
pub struct WasmSnapshot {
    bucket_id: ApiDriveId,
    snapshot: ApiSnapshot,
}

impl WasmSnapshot {
    pub(crate) fn new(bucket_id: ApiDriveId, snapshot: ApiSnapshot) -> Self {
        Self {
            bucket_id,
            snapshot,
        }
    }
}

#[wasm_bindgen]
impl WasmSnapshot {
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        self.bucket_id.clone()
    }

    // note(sstelfox): This was returning a string... but it looks like it might've just been
    // stringifying the number...
    #[wasm_bindgen(getter = createdAt)]
    pub fn created_at(&self) -> i64 {
        self.snapshot.created_at()
    }

    pub fn id(&self) -> String {
        self.snapshot.id()
    }

    #[wasm_bindgen(js_name = metadataId)]
    pub fn metadata_id(&self) -> String {
        self.snapshot.metadata_id()
    }

    // note(sstelfox): We may not know the size of the snapshot, this should have some concept of
    // optional instead of 0
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> i64 {
        self.snapshot.size().unwrap_or(0)
    }
}
