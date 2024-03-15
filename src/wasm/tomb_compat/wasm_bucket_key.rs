use wasm_bindgen::prelude::*;

use crate::api::platform::{ApiDriveId, ApiDriveKey};

#[wasm_bindgen]
pub struct WasmBucketKey(ApiDriveId, ApiDriveKey);

#[wasm_bindgen]
impl WasmBucketKey {
    // note(sstelfox): og version didn't have this attr
    #[wasm_bindgen(getter)]
    pub fn approved(&self) -> bool {
        self.1.approved()
    }

    // note(sstelfox): og version used the following:
    //#[wasm_bindgen(js_name = bucketId)]
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        self.0.clone()
    }

    // note(sstelfox): og version didn't expose this at all
    #[wasm_bindgen(getter)]
    pub fn fingerprint(&self) -> String {
        self.1.fingerprint().into()
    }

    // note(sstelfox): didn't have this
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.1.id().clone()
    }

    // note(sstelfox): had this other attr
    //#[wasm_bindgen(js_name = pem)]
    #[wasm_bindgen(getter = publicKey)]
    pub fn public_key(&self) -> String {
        self.1.public_key().into()
    }
}

impl From<(ApiDriveId, ApiDriveKey)> for WasmBucketKey {
    fn from((drive_id, drive_key): (ApiDriveId, ApiDriveKey)) -> Self {
        WasmBucketKey(drive_id, drive_key)
    }
}
