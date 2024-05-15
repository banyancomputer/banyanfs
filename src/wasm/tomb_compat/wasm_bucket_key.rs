use crate::api::platform::{ApiDriveAccess, ApiDriveId, ApiKeyId};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmBucketAccess(ApiDriveAccess);

#[wasm_bindgen]
impl WasmBucketAccess {
    #[wasm_bindgen(getter = driveId)]
    pub fn drive_id(&self) -> ApiDriveId {
        self.0.drive_id().to_string()
    }

    #[wasm_bindgen(getter)]
    #[wasm_bindgen(js_name = userKeyId)]
    pub fn user_key_id(&self) -> ApiKeyId {
        self.0.user_key_id().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn approved(&self) -> bool {
        self.0.approved()
    }

    #[wasm_bindgen(getter)]
    pub fn fingerprint(&self) -> String {
        self.0.fingerprint().to_string()
    }
}

impl From<ApiDriveAccess> for WasmBucketAccess {
    fn from(value: ApiDriveAccess) -> Self {
        Self(value)
    }
}
