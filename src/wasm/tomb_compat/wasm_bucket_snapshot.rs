use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmBucketSnapshot;

#[wasm_bindgen]
impl WasmBucketSnapshot {
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        todo!()
    }

    #[wasm_bindgen(getter = createdAt)]
    pub fn created_at(&self) -> i64 {
        todo!()
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        todo!()
    }

    #[wasm_bindgen(getter = metadataId)]
    pub fn metadata_id(&self) -> String {
        todo!()
    }
}
