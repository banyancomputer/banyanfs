use async_trait::async_trait;
use url::Url;

use crate::{codec::Cid, prelude::ContentLocation};

/// The core storage trait for the library backend. This is the minimum requirement for
/// integrating banyanfs with any form of external storage or platform. Data availability and
/// reliability relies on the implementation of this trait and the backing store. Failing to save
/// data will not corrupt the filesystem itself but will prevent the affected file(s) from being
/// readable.
///
/// This store does not receive everything necessary about the the filesystem it only includes the
/// encrypted data blocks backing files. The filesystem itself needs to be encoded and stored
/// separately. Refer the examples for how to persist the metadata and access layer of the
/// filesystem.
#[async_trait(?Send)]
pub trait DataStore {
    /// Check if the implementor of this trait knows about and is able to retrieve the provided
    /// CID. Only accepts the internal CID type which is strict in its interpretation as the
    /// BanyanFS has explicitly standardized on the BLAKE3 hash algorithm.
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError>;

    /// Allows the filesystem to request the removal of a block. Once done it assumes the block is
    /// no longer available and will not attempt to retrieve it unless the block gets added back to
    /// the filesystem.
    async fn remove(&mut self, cid: Cid, recursive: bool) -> Result<(), DataStoreError>;

    /// Retrieve the data block associated with the provided CID. The implementor should return the
    /// raw bytes of the complete block with the data header intact.
    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError>;

    /// Store a complete block, the format of which is defined by BanyanFS but is compatible to be
    /// wrapped in a CAR file if so chosen (Standard CIDv1 multicodec with a RAW codec and the
    /// Blake3 hash function is used for the block).
    ///
    /// If adopted future versions of the filesystem will register and store/expect a dedicated
    /// codec identity for the blocks but will continue to accept and retrieve blocks formatted
    /// with the RAW codec.
    ///
    /// The immediate flag is a hint that the data should be persisted immediately rather than just
    /// in any form of a cache. It's up to the implementor to decide what the exact behavior of
    /// this flag is. Expects the block to be immediately available for retrieval once this call
    /// completes successfully.
    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        immediate: bool,
    ) -> Result<(), DataStoreError>;
}

pub trait PackableStore: SyncableDataStore {
    async fn store_packed(
        &mut self,
        data: Vec<u8>,
    ) -> Result<Vec<ContentReference>, DataStoreError>;
}

pub struct ContentReference {
    data_block_cid: Cid,
    chunks: Vec<ContentLocation>,
}

pub enum BlockCid {
    Cid(Cid),
    Unresolved, // Does this need some sort of identifier?
}

/// An optional additional trait for implementors that would like to perform periodic flushing of
/// storage instead of reading and writing every block immediately. This is particularly useful in
/// aggregating filesystem operations for one large write, splitting up sync operations into
/// multiple smaller writer, or implementing a cache layer over another block store.
#[async_trait(?Send)]
pub trait SyncableDataStore: DataStore + SyncTracker {
    /// It is expected that syncable data stores may need to retrieve data from different hosts.
    /// This is intended as a mechanism to switch the default host to retrieve or store blocks
    /// with. Actual behavior is up to the implementor.
    async fn set_sync_host(&mut self, host: Url) -> Result<(), DataStoreError>;

    /// A variant of store that should immediate sync. This is conceptually similar to an fsync on
    /// a standard filesystem but limited only to the block being stored. By default this makes use
    /// of the immediate flag to the normal store method.
    async fn store_sync(&mut self, cid: Cid, data: Vec<u8>) -> Result<(), DataStoreError> {
        self.store(cid, data, true).await
    }

    /// Indicates that any cached or non-persisted blocks should be immediately persisted. Similar
    /// to a fsync operation on a standard filesystem. Failure will occur if any of the blocks fail
    /// to write but it does not necessarily indicate that all failed. The implementor of this
    /// trait should be careful to not repeat work or redo block storage that has already
    /// succeeded.
    async fn sync(&mut self, version_id: &str) -> Result<(), DataStoreError>;

    /// Returns the amount of dirty / unsynced data that currently resides on the store. This will
    /// be a combination of the size of the blocks used and does not necessarily represent the
    /// underlying size of data changes.
    async fn unsynced_data_size(&self) -> Result<u64, DataStoreError> {
        self.tracked_size().await
    }
}

/// This trait tracks which blocks are stored where in a distributed block storage system.
///
/// A required trait for implementors of the SyncableDataStore trait, this trait has a set of
/// common operations for tracking which blocks live where. This trait is likely not needed as
/// it is heavily impacting the design of anyone implementing the SyncableDataStore.
#[async_trait(?Send)]
pub trait SyncTracker {
    /// Clears the list of all blocks that have been indicated they've been deleted. Intended use
    /// is for immediately after notifying a remote system that a set of blocks are no longer
    /// needed.
    async fn clear_deleted(&mut self) -> Result<(), DataStoreError>;

    /// Indicate the provide CID is no longer needed and can be removed from the store but does not
    /// sync this information on its own.
    async fn delete(&mut self, cid: Cid) -> Result<(), DataStoreError>;

    /// Returns he currently tracked list of CIDs that have been marked for deletion.
    async fn deleted_cids(&self) -> Result<Vec<Cid>, DataStoreError>;

    /// Track a provided CID indicating that it still needs to be synced/persisted. The reported
    /// size is used for needed storage calculations and can be accessed through the
    /// [`SyncTracker::tracked_size`] method.
    async fn track(&mut self, cid: Cid, size: u64) -> Result<(), DataStoreError>;

    /// Returns all the CIDs that haven't currently been persisted.
    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError>;

    /// Returns the total size of all the tracked CIDs that have been marked for storage. Useful
    /// when selecting where the data will be persisted.
    async fn tracked_size(&self) -> Result<u64, DataStoreError>;

    /// Allows marking individual CIDs as no longer needing to be tracked. Useful for stores to to
    /// perform incremental block-based synchronization that keeps track of its ongoing state.
    async fn untrack(&mut self, cid: Cid) -> Result<(), DataStoreError>;
}

/// Various common errors that can generically
#[derive(Debug, thiserror::Error)]
pub enum DataStoreError {
    /// An error that couldn't be represented by one of the standard error types, representing some
    /// kind of error specific to the underlying implementation.
    #[error("implementation specific error: {0}")]
    Implementation(String),

    /// The requested block is not available in the store, and none of its available data sources
    /// is aware of the block either. Stores should exhaust all sources before returning this
    /// error.
    #[error("failed to retrieve block")]
    LookupFailure,

    /// For operations that require a default storage host to be set but didn't have one avaiable
    /// will return this error. The caller should ensure that they have provided a valid host to
    /// the store before calling the operation again.
    #[error("no storage hosts have been registered to interact with")]
    NoActiveStorageHost,

    /// The store knew about the block but failed to actually retreive the block containing the
    /// data. This error may be permanent or ephemeral, the caller will need to use additional
    /// details to determine.
    #[error("failed to retreive block from network")]
    RetrievalFailure,

    /// Should use this error primarily for authentication or authorization failures. The active
    /// session the store currently had was invalid or additional authentication is needed. This
    /// takes priority over other forms of errors when the root cause is based on authentication or
    /// authorization.
    #[error("failed to open storage session")]
    SessionRejected,

    /// The store received the block but failed to store it, or if the immediate flag was passed
    /// may have failed to immediately persist it. It should be safe for the caller to retry the
    /// operation after resolving the underlying issue.
    #[error("failed to store block")]
    StoreFailure,
}
