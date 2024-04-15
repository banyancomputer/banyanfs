#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "banyan-api")]
pub mod api;

pub mod codec;
pub mod error;
pub mod filesystem;
pub mod stores;
pub mod utils;
pub mod version;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Re-export of this trait in particular can be very useful for downstream users and matches other
// common crates such as axum that do the same.
pub use async_trait;

/// Prelude for the banyanfs library exporting the most commonly used types and traits.
///
/// ```rust
/// use banyanfs::prelude::*;
/// ```
pub mod prelude {
    #[cfg(feature = "banyan-api")]
    pub use crate::api::ApiClient;
    pub use crate::codec::crypto::{SigningKey, VerifyingKey};
    pub use crate::codec::header::ContentOptions;
    pub use crate::codec::FilesystemId;
    pub use crate::error::{BanyanFsError, BanyanFsResult};
    pub use crate::filesystem::{
        DirectoryEntry, DirectoryHandle, Drive, DriveAccess, DriveLoader, DriveLoaderError,
        OperationError,
    };
    pub use crate::stores::{
        ApiSyncableStore, DataStore, DataStoreError, MemoryDataStore, MemorySyncTracker,
        SyncTracker, SyncableDataStore,
    };
    pub use crate::version::{full_version, minimal_version, user_agent_byte_str};
}
