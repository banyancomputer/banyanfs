pub(crate) mod models;
mod wasm_bucket;
mod wasm_bucket_key;
mod wasm_bucket_metadata;
mod wasm_bucket_mount;
mod wasm_fs_metadata_entry;
mod wasm_mount;
mod wasm_node_metadata;
mod wasm_shared_file;
mod wasm_snapshot;
mod wasm_user_key;
mod wasm_user_key_access;

pub use wasm_bucket::WasmBucket;
pub use wasm_bucket_key::WasmBucketAccess;
pub use wasm_bucket_metadata::WasmBucketMetadata;
pub use wasm_bucket_mount::WasmBucketMount;
pub use wasm_fs_metadata_entry::WasmFsMetadataEntry;
pub use wasm_mount::WasmMount;
pub use wasm_node_metadata::WasmNodeMetadata;
pub use wasm_shared_file::WasmSharedFile;
pub use wasm_snapshot::WasmSnapshot;
pub use wasm_user_key::WasmUserKey;
pub use wasm_user_key_access::WasmUserKeyAccess;

use std::str::FromStr;
use std::sync::Arc;

use tracing::debug;
use wasm_bindgen::prelude::*;
use zeroize::Zeroize;

// This section should effectively be a consumer of the normal API calls, nothing in here should
// have a namespace conflict with anything exported in the prelude, as nothing in this or any
// submodules should be exported in the prelude.
use crate::api::platform;
use crate::api::platform::{DriveKind, StorageClass};
use crate::prelude::*;
use crate::wasm::data_storage::{initialize_store, WasmDataStorage};

use models::TombBucket;

#[derive(Clone)]
#[wasm_bindgen(js_name = TombWasm)]
pub struct TombCompat {
    client: ApiClient,
    key: Arc<SigningKey>,
    store: WasmDataStorage,
}

impl TombCompat {
    pub(crate) fn client(&self) -> &ApiClient {
        &self.client
    }

    pub(crate) fn store(&self) -> WasmDataStorage {
        self.store.clone()
    }

    pub(crate) fn signing_key(&self) -> Arc<SigningKey> {
        self.key.clone()
    }
}

#[wasm_bindgen(js_class = TombWasm)]
impl TombCompat {
    // new transfered and checked
    #[wasm_bindgen(js_name = createUserKey)]
    pub async fn create_user_key(
        &mut self,
        name: String,
        public_pem: String,
    ) -> BanyanFsResult<()> {
        let public_key = match VerifyingKey::from_spki(&public_pem) {
            Ok(key) => key,
            Err(err) => return Err(format!("failed to load public key: {err}").into()),
        };

        tracing::warn!("using broken API key registration, needs to be reworked");
        platform::account::create_user_key(&self.client, &name, &public_key).await?;

        Ok(())
    }

    #[wasm_bindgen(js_name = revokeBucketAccess)]
    pub async fn revoke_bucket_access(
        &mut self,
        bucket_id: String,
        fingerprint: String,
    ) -> BanyanFsResult<()> {
        platform::drive_access::revoke(&self.client, &bucket_id, &fingerprint).await?;
        Ok(())
    }

    #[wasm_bindgen(js_name = userKeyAccess)]
    pub async fn user_key_access(&mut self) -> BanyanFsResult<js_sys::Array> {
        Ok(platform::account::user_key_access(&self.client)
            .await?
            .into_iter()
            .map(|uka| JsValue::from(WasmUserKeyAccess(uka)))
            .collect::<js_sys::Array>())
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
    //}

    // new transfered and checked,
    //
    // note(sstelfox): we already have the private key, and that gives us the public key. I'm
    // checking this to make sure there isn't any errors but we can drop these parameters. It never
    // makes sense to create something other than a hot bucket so that can be removed, and in
    // practice we only support interactive buckets. I'm going to validate them here but this API
    // should be changed.
    #[wasm_bindgen(js_name = createBucketAndMount)]
    pub async fn create_bucket_and_mount(
        &mut self,
        name: String,
        storage_class: String,
        bucket_type: String,
        mut private_key_pem: String,
        public_key_pem: String,
    ) -> BanyanFsResult<WasmBucketMount> {
        let private_key = match SigningKey::from_pkcs8_pem(&private_key_pem) {
            Ok(key) => Arc::new(key),
            Err(err) => return Err(format!("failed to load private key: {err}").into()),
        };
        private_key_pem.zeroize();

        if self.key.key_id() != private_key.key_id() {
            tracing::warn!(init_key_id = ?self.key.key_id(), private_key_id = ?private_key.key_id(), "provided private key doesn't match initialized webkey");
            //return Err("provided private key doesn't match initialized webkey".into());
        }

        let public_key = match VerifyingKey::from_spki(&public_key_pem) {
            Ok(key) => key,
            Err(err) => return Err(format!("failed to load public key: {err}").into()),
        };

        if private_key.key_id() != public_key.key_id() {
            return Err("provided public key doesn't match provided private key".into());
        }

        // Just confirm they're valid and the kind we support
        let sc = StorageClass::from_str(&storage_class)?;
        if sc != StorageClass::Hot {
            return Err("only hot storage is allowed to be created".into());
        }

        let dk = DriveKind::from_str(&bucket_type)?;
        if dk != DriveKind::Interactive {
            return Err("only interactive buckets are allowed to be created".into());
        }

        let id = platform::drives::create(&self.client, &name, &public_key).await?;

        let wasm_bucket = WasmBucket(TombBucket::from_components(id.clone(), name, sc, dk));
        let wasm_mount =
            WasmMount::initialize(wasm_bucket.clone(), self.clone(), self.store.clone()).await?;

        Ok(WasmBucketMount::new(wasm_bucket, wasm_mount))
    }

    // checked, no return
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket(&mut self, bucket_id: String) -> BanyanFsResult<()> {
        platform::drives::delete(&self.client, &bucket_id).await?;
        Ok(())
    }

    // checked, returns Account::usage response, changed return type from u64 to usize
    #[wasm_bindgen(js_name = getUsage)]
    pub async fn get_usage(&mut self) -> BanyanFsResult<usize> {
        let current_usage = platform::account::current_usage(&self.client).await?;
        Ok(current_usage.total_usage())
    }

    // checked, returns Account::usage_limit response, changed return type from u64 to usize
    #[wasm_bindgen(js_name = getUsageLimit)]
    pub async fn get_usage_limit(&mut self) -> BanyanFsResult<usize> {
        let current_usage = platform::account::current_usage_limit(&self.client).await?;
        Ok(current_usage.soft_hot_storage_limit())
    }

    // checked, returns list of WasmBucket instances
    // note(sstelfox): change return type from js_sys::Array to JsValue should be compatible but
    // seems to be fine so far
    #[wasm_bindgen(js_name = listBuckets)]
    pub async fn list_buckets(&mut self) -> BanyanFsResult<js_sys::Array> {
        let all_drives = platform::drives::get_all(&self.client).await?;

        let buckets = all_drives
            .into_iter()
            .map(models::TombBucket::from)
            .map(|tb| JsValue::from(WasmBucket(tb)))
            .collect::<js_sys::Array>();

        Ok(buckets)
    }

    // checked, returns list of WasmBucketKey instances
    #[wasm_bindgen(js_name = listBucketAccess)]
    pub async fn list_bucket_access(&mut self, bucket_id: String) -> BanyanFsResult<js_sys::Array> {
        let all_drive_access = platform::drive_access::get_all(&self.client, &bucket_id).await?;

        let bucket_access = all_drive_access
            .into_iter()
            .map(|da| WasmBucketAccess::from(da))
            .map(JsValue::from)
            .collect::<js_sys::Array>();

        Ok(bucket_access)
    }

    // checked, returns list of WasmSnapshot instances
    #[wasm_bindgen(js_name = listBucketSnapshots)]
    pub async fn list_bucket_snapshots(
        &mut self,
        bucket_id: String,
    ) -> BanyanFsResult<js_sys::Array> {
        let snapshots = platform::snapshots::get_all(&self.client, &bucket_id).await?;

        let js_snaps = snapshots
            .into_iter()
            .map(|s| JsValue::from(WasmSnapshot::new(bucket_id.clone(), s)))
            .collect::<js_sys::Array>();

        Ok(js_snaps)
    }

    // checked, returns WasmMount instance
    #[wasm_bindgen(js_name = mount)]
    pub async fn mount(
        &mut self,
        drive_id: String,
        mut private_key_pem: String,
    ) -> BanyanFsResult<WasmMount> {
        let private_key = match SigningKey::from_pkcs8_pem(&private_key_pem) {
            Ok(key) => Arc::new(key),
            Err(err) => return Err(format!("failed to load private key: {err}").into()),
        };
        private_key_pem.zeroize();

        if self.key.key_id() != private_key.key_id() {
            tracing::warn!(init_key_id = ?self.key.key_id(), private_key_id = ?private_key.key_id(), "provided private key doesn't match initialized webkey");
            //return Err("provided private key doesn't match initialized webkey".into());
        }

        let drive = platform::drives::get(&self.client, &drive_id).await?;
        let wasm_bucket = WasmBucket(TombBucket::from(drive));

        let mount = WasmMount::pull(wasm_bucket.clone(), self.clone()).await?;

        // note(sstelfox): the old version attempts to unlock the mount here, but I've migrated that into the
        // pull itself if the key matches. This mount will only fail if the bucket doesn't have any
        // metadata or the current key is incorrect.
        //
        // Given this is an explicit mount operation it likely should be an error if we're unable
        // to do so...

        Ok(mount)
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
        debug!(account_id, key_id = ?key.key_id(), "initialized new TombWasm instance");

        let client = ApiClient::new(&api_endpoint, &account_id, key.clone())
            .expect("need return type fixed");

        let store = initialize_store(client.clone());

        Self { client, key, store }
    }

    // new transfered and checked
    #[wasm_bindgen(js_name = renameBucket)]
    pub async fn rename_bucket(&mut self, bucket_id: String, name: String) -> BanyanFsResult<()> {
        let attrs = platform::ApiDriveUpdateAttributes { name: Some(name) };
        platform::drives::update(&self.client, &bucket_id, attrs).await?;
        Ok(())
    }
}
