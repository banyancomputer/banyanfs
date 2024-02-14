use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmBucketKey;

#[wasm_bindgen]
impl WasmBucketKey {
    //#[wasm_bindgen(getter)]
    pub fn approved(&self) -> bool {
        todo!()
    }

    //#[wasm_bindgen(getter = bucketId)]
    #[wasm_bindgen(js_name = bucketId)]
    pub fn bucket_id(&self) -> String {
        todo!()
    }

    //#[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        todo!()
    }

    //#[wasm_bindgen(getter = publicKey)]
    #[wasm_bindgen(js_name = pem)]
    pub fn public_key(&self) -> String {
        todo!()
    }
}
