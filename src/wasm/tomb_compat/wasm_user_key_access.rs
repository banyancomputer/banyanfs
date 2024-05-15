use crate::api::platform::ApiUserKeyAccess;
use crate::wasm::WasmUserKey;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmUserKeyAccess(pub(crate) ApiUserKeyAccess);

impl From<ApiUserKeyAccess> for WasmUserKeyAccess {
    fn from(value: ApiUserKeyAccess) -> Self {
        Self(value)
    }
}

#[wasm_bindgen]
impl WasmUserKeyAccess {
    #[wasm_bindgen(getter = key)]
    pub fn key(&self) -> WasmUserKey {
        WasmUserKey(self.0.key.clone())
    }

    #[wasm_bindgen(getter = bucketIds)]
    pub fn bucket_ids(&self) -> js_sys::Array {
        self.0
            .bucket_ids
            .clone()
            .into_iter()
            .map(JsValue::from)
            .collect::<js_sys::Array>()
    }
}
