//! # Stores
//!
//! BanyanFS on its own delegates the storing of the blocks representing file data to
//! implementations of the [`DataStore`] trait. This allows for a wide variety of storage
//! options from the official [Banyan Platform](https://banyan.computer), to local disk storage, or
//! a custom block storage system.

mod api_syncable_store;
#[cfg(feature = "local-store")]
mod local_data_store;
mod memory_data_store;
mod memory_sync_tracker;
mod traits;

pub use api_syncable_store::ApiSyncableStore;
#[cfg(feature = "local-store")]
pub use local_data_store::LocalDataStore;
pub use memory_data_store::MemoryDataStore;
pub use memory_sync_tracker::MemorySyncTracker;
pub use traits::{DataStore, DataStoreError, SyncTracker, SyncableDataStore};
