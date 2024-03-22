use wasm_bindgen::prelude::*;

//use crate::wasm::tomb_compat::{WasmBucket, WasmMount};

#[wasm_bindgen]
pub struct WasmSharedFile;

#[wasm_bindgen]
impl WasmSharedFile {
    #[wasm_bindgen(js_name = fileName)]
    pub fn file_name(&self) -> String {
        tracing::warn!("impl needed shared file name");
        "not currently implemented".to_string()
    }

    #[wasm_bindgen(js_name = mimeType)]
    pub fn mime_type(&self) -> String {
        tracing::warn!("impl needed mime type");
        "not currently implemented".to_string()
    }
}
