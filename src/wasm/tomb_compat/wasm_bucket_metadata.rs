use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmBucketMetadata;

#[wasm_bindgen]
impl WasmBucketMetadata {
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        todo!()
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        todo!()
    }

    #[wasm_bindgen(getter = snapshotId)]
    pub fn snapshot_id(&self) -> String {
        // todo: needs to become an option type
        todo!()
    }
}
