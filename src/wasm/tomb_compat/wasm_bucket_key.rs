use wasm_bindgen::prelude::*;

use crate::api::platform::ApiDriveKey;

#[wasm_bindgen]
pub struct WasmBucketKey {
    id: String,
    bucket_id: String,

    fingerprint: String,
    public_key: String,

    approved: bool,
}

#[wasm_bindgen]
impl WasmBucketKey {
    // note(sstelfox): og version didn't have this attr
    #[wasm_bindgen(getter)]
    pub fn approved(&self) -> bool {
        self.approved
    }

    // note(sstelfox): og version used the following:
    //#[wasm_bindgen(js_name = bucketId)]
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        self.bucket_id.to_string()
    }

    // note(sstelfox): og version didn't expose this at all
    #[wasm_bindgen(getter)]
    pub fn fingerprint(&self) -> String {
        self.fingerprint.clone()
    }

    // note(sstelfox): didn't have this
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    // note(sstelfox): had this other attr
    //#[wasm_bindgen(js_name = pem)]
    #[wasm_bindgen(getter = publicKey)]
    pub fn public_key(&self) -> String {
        self.public_key.clone()
    }
}

impl From<ApiDriveKey> for WasmBucketKey {
    fn from(api_drive_key: ApiDriveKey) -> Self {
        WasmBucketKey {
            id: api_drive_key.id().to_string(),
            bucket_id: api_drive_key.drive_id().to_string(),

            fingerprint: api_drive_key.fingerprint().to_string(),
            public_key: api_drive_key.public_key().to_string(),

            approved: api_drive_key.approved(),
        }
    }
}
