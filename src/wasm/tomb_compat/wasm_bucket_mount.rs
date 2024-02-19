use wasm_bindgen::prelude::*;

use crate::wasm::tomb_compat::{WasmBucket, WasmMount};

#[wasm_bindgen]
pub struct WasmBucketMount {
    bucket: WasmBucket,
    mount: WasmMount,
}

#[wasm_bindgen]
impl WasmBucketMount {
    #[wasm_bindgen(getter)]
    pub fn bucket(&self) -> WasmBucket {
        self.bucket.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn mount(&self) -> WasmMount {
        self.mount.clone()
    }

    #[wasm_bindgen]
    pub fn new(bucket: WasmBucket, mount: WasmMount) -> Self {
        Self { bucket, mount }
    }
}
