use wasm_bindgen::prelude::*;

use crate::wasm::tomb_compat::models::TombBucket;

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmBucket(pub(crate) TombBucket);

#[wasm_bindgen]
impl WasmBucket {
    #[wasm_bindgen(js_name = bucketType)]
    pub fn bucket_type(&self) -> String {
        todo!()
    }

    pub fn id(&self) -> String {
        todo!()
    }

    pub fn name(&self) -> String {
        todo!()
    }

    #[wasm_bindgen(js_name = storageClass)]
    pub fn storage_class(&self) -> String {
        todo!()
    }
}
