use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmBucket;

#[wasm_bindgen]
impl WasmBucket {
    //#[wasm_bindgen(getter)]
    #[wasm_bindgen(js_name = bucketType)]
    pub fn bucket_type(&self) -> String {
        todo!()
    }

    //#[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        todo!()
    }

    //#[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        todo!()
    }

    //#[wasm_bindgen(getter = storageClass)]
    #[wasm_bindgen(js_name = storageClass)]
    pub fn storage_class(&self) -> String {
        todo!()
    }
}
