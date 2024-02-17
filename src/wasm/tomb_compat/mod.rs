mod wasm_bucket;
mod wasm_bucket_key;
mod wasm_bucket_metadata;
mod wasm_bucket_mount;
mod wasm_fs_metadata_entry;
mod wasm_mount;
mod wasm_node_metadata;
mod wasm_shared_file;
mod wasm_snapshot;

pub use wasm_bucket::WasmBucket;
pub use wasm_bucket_key::WasmBucketKey;
pub use wasm_bucket_metadata::WasmBucketMetadata;
pub use wasm_bucket_mount::WasmBucketMount;
pub use wasm_fs_metadata_entry::WasmFsMetadataEntry;
pub use wasm_mount::WasmMount;
pub use wasm_node_metadata::WasmNodeMetadata;
pub use wasm_shared_file::WasmSharedFile;
pub use wasm_snapshot::WasmSnapshot;

use std::sync::Arc;

use reqwest::Url;
use tracing::debug;
use wasm_bindgen::prelude::*;
use zeroize::Zeroize;

// This section should effectively be a consumer of the normal API calls, nothing in here should
// have a namespace conflict with anything exported in the prelude, as nothing in this or any
// submodules should be exported in the prelude.
use crate::prelude::*;

#[wasm_bindgen(js_name = TombWasm)]
pub struct TombCompat {
    client: ApiClient,
    key: Arc<SigningKey>,
}

#[wasm_bindgen(js_class = TombWasm)]
impl TombCompat {
    // new transfered and checked
    #[wasm_bindgen(js_name = approveDeviceApiKey)]
    pub async fn approve_device_api_key(&mut self, _pem: String) -> BanyanFsResult<()> {
        todo!()
    }

    // appears to no longer be present, likely migrated to create_bucket_and_mount
    //#[wasm_bindgen(js_name = createBucket)]
    //pub async fn create_bucket(
    //    &mut self,
    //    name: String,
    //    storage_class: String,
    //    bucket_type: String,
    //    public_key: CryptoKey,
    //) -> BanyanFsResult<WasmBucket> {
    //    todo!()
    //}

    // new transfered and checked,
    #[wasm_bindgen(js_name = createBucketAndMount)]
    pub async fn create_bucket_and_mount(
        &mut self,
        _name: String,
        _storage_class: String,
        _bucket_type: String,
        _private_pem: String,
        _public_pem: String,
    ) -> BanyanFsResult<WasmBucketMount> {
        todo!()
    }

    // checked, returns WasmBucketKey instance
    #[wasm_bindgen(js_name = createBucketKey)]
    pub async fn create_bucket_key(&mut self, _bucket_id: String) -> BanyanFsResult<WasmBucketKey> {
        todo!()
    }

    // checked, no return
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket(&mut self, _bucket_id: String) -> BanyanFsResult<()> {
        todo!()
    }

    // checked, returns Account::usage response
    #[wasm_bindgen(js_name = getUsage)]
    pub async fn get_usage(&mut self) -> BanyanFsResult<u64> {
        todo!()
    }

    // checked, returns Account::usage_limit response
    #[wasm_bindgen(js_name = getUsageLimit)]
    pub async fn get_usage_limit(&mut self) -> BanyanFsResult<u64> {
        todo!()
    }

    // checked, returns list of WasmBucket instances
    #[wasm_bindgen(js_name = listBuckets)]
    pub async fn list_buckets(&mut self) -> BanyanFsResult<js_sys::Array> {
        todo!()
    }

    // checked, returns list of WasmBucketKey instances
    #[wasm_bindgen(js_name = listBucketKeys)]
    pub async fn list_bucket_keys(&mut self, _bucket_id: String) -> BanyanFsResult<js_sys::Array> {
        todo!()
    }

    // checked, returns list of WasmSnapshot instances
    #[wasm_bindgen(js_name = listBucketSnapshots)]
    pub async fn list_bucket_snapshots(
        &mut self,
        _bucket_id: String,
    ) -> BanyanFsResult<js_sys::Array> {
        todo!()
    }

    // checked, returns WasmMount instance
    #[wasm_bindgen(js_name = mount)]
    pub async fn mount(
        &mut self,
        _bucket_id: String,
        _key_pem: String,
    ) -> BanyanFsResult<WasmMount> {
        todo!()
    }

    // checked, returns itself, DANGER: needs to be fallible
    #[wasm_bindgen(constructor)]
    pub async fn new(
        mut private_key_pem: String,
        account_id: String,
        api_endpoint: String,
    ) -> Self {
        let key = match SigningKey::from_pkcs8_pem(&private_key_pem) {
            Ok(key) => Arc::new(key),
            Err(e) => panic!("Failed to create signing key: {}", e),
        };
        private_key_pem.zeroize();

        let api_endpoint = match reqwest::Url::parse(&api_endpoint) {
            Ok(url) => url,
            Err(e) => panic!("Failed to parse api endpoint: {}", e),
        };

        let client = ApiClient::with_auth(api_endpoint, account_id, key.clone());

        debug!(key_id = ?key.key_id(), "initialized new TombWasm instance");

        Self { client, key }
    }

    // new transfered and checked
    #[wasm_bindgen(js_name = renameBucket)]
    pub async fn rename_bucket(&mut self, _bucket_id: String, _name: String) -> BanyanFsResult<()> {
        todo!()
    }
}
