use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmSnapshot;

#[wasm_bindgen]
impl WasmSnapshot {
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        todo!()
    }

    #[wasm_bindgen(getter = createdAt)]
    pub fn created_at(&self) -> String {
        todo!()
    }

    pub fn id(&self) -> String {
        todo!()
    }

    #[wasm_bindgen(js_name = metadataId)]
    pub fn metadata_id(&self) -> String {
        todo!()
    }

    #[wasm_bindgen(getter)]
    pub fn size(&self) -> i64 {
        todo!()
    }
}
