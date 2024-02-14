use wasm_bindgen::prelude::*;

use crate::wasm::tomb_compat::{WasmBucket, WasmMount};

#[wasm_bindgen]
pub struct WasmBucketMount;

#[wasm_bindgen]
impl WasmBucketMount {
    #[wasm_bindgen(getter)]
    pub fn bucket(&self) -> WasmBucket {
        todo!()
    }

    #[wasm_bindgen(getter)]
    pub fn mount(&self) -> WasmMount {
        todo!()
    }

    //#[wasm_bindgen(constructor)]
    #[wasm_bindgen]
    pub fn new(_bucket: WasmBucket, _mount: WasmMount) -> Self {
        todo!()
    }
}
