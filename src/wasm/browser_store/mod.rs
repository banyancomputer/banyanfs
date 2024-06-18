mod indexed_db_sync_tracker;
mod web_fs_data_store;

use crate::api::ApiClient;
use crate::stores::ApiSyncableStore;

pub use indexed_db_sync_tracker::IndexedDbSyncTracker;
pub use web_fs_data_store::WebFsDataStore;

// todo(sstelfox): I really want WASM aware versions of this that can be shared between browser
// tabs, likely this means using the filesystem for the data store and indexdb for the sync
// tracker.
pub type BrowserStore = ApiSyncableStore<WebFsDataStore, IndexedDbSyncTracker>;

pub fn initialize_browser_store(api_client: ApiClient) -> BrowserStore {
    ApiSyncableStore::new(api_client, WebFsDataStore, IndexedDbSyncTracker)
}
