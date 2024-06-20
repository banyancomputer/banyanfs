mod indexed_db_sync_tracker;
mod web_fs_data_store;

use crate::api::ApiClient;
use crate::stores::ApiSyncableStore;

pub use indexed_db_sync_tracker::{IndexedDbSyncTracker, IndexedDbSyncTrackerError};
pub use web_fs_data_store::{WebFsDataStore, WebFsDataStoreError};

pub type BrowserStore = ApiSyncableStore<WebFsDataStore, IndexedDbSyncTracker>;

pub async fn initialize_browser_store(
    api_client: ApiClient,
) -> Result<BrowserStore, BrowserStoreError> {
    let fs_store = WebFsDataStore::new().await?;
    let idb_tracker = IndexedDbSyncTracker::new().await?;

    Ok(ApiSyncableStore::new(api_client, fs_store, idb_tracker))
}

/// This is an equivalent version of web_sys::window() but grabs a web worker's scope instead. If
/// the code isn't running in a worker context this will return None.
pub(crate) fn worker() -> Option<web_sys::WorkerGlobalScope> {
    use wasm_bindgen::JsCast;

    js_sys::global()
        .dyn_into::<web_sys::WorkerGlobalScope>()
        .ok()
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserStoreError {
    #[error("error in web filesystem data store: {0}")]
    WebFsError(#[from] WebFsDataStoreError),

    #[error("error in IndexedDB sync tracker: {0}")]
    SyncTrackerError(#[from] IndexedDbSyncTrackerError),
}
