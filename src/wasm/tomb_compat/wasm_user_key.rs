use crate::api::platform::ApiUserKey;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmUserKey(pub(crate) ApiUserKey);

#[wasm_bindgen]
impl WasmUserKey {
    /// Key Id
    pub fn id(&self) -> String {
        self.0.id().to_string()
    }

    /// Name of the Key
    pub fn name(&self) -> String {
        self.0.name().to_string()
    }

    /// User Id of the Owner of the Key
    #[wasm_bindgen(js_name = userId)]
    pub fn user_id(&self) -> String {
        self.0.user_id().to_string()
    }

    /// API usability
    #[wasm_bindgen(js_name = apiAccess)]
    pub fn api_access(&self) -> bool {
        self.0.api_access()
    }

    /// PEM
    pub fn pem(&self) -> String {
        self.0.pem().to_string()
    }

    /// Public Key Fingerprint
    pub fn fingerprint(&self) -> String {
        self.0.fingerprint().to_string()
    }
}
