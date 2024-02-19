use wasm_bindgen::prelude::*;

use crate::wasm::tomb_compat::models::TombBucket;

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmBucket(pub(crate) TombBucket);

#[wasm_bindgen]
impl WasmBucket {
    #[wasm_bindgen(js_name = bucketType)]
    pub fn bucket_type(&self) -> String {
        self.0.kind()
    }

    pub fn id(&self) -> String {
        self.0.id()
    }

    pub fn name(&self) -> String {
        self.0.name()
    }

    #[wasm_bindgen(js_name = storageClass)]
    pub fn storage_class(&self) -> String {
        self.0.storage_class()
    }
}
