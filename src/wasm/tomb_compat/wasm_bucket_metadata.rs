use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmBucketMetadata(ApiDriveId, ApiMetadataId);

#[wasm_bindgen]
impl WasmBucketMetadata {
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        todo!()
    }

    // note(sstelfox): this is a metadata ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        todo!()
    }

    // note(sstelfox): this is wrong, snapshot's aren't guaranteed to be present, this needs t be
    // an Option, for now we'll return an empty string when not present
    #[wasm_bindgen(getter = snapshotId)]
    pub fn snapshot_id(&self) -> String {
        todo!()
    }
}
