use std::sync::Arc;

use parking_lot::{
    lock_api::RwLockWriteGuard, MappedRwLockReadGuard, RwLock, RwLockReadGuard,
    RwLockUpgradableReadGuard,
};

/// Cache that stores a copy of the encoding of a node. Encoding is used to get the CID of a node, to get its size,
/// and to serialize the node when writing it to disk (or network, etc.). If anything about the node changes this will
/// be marked dirty so it can be recomputed. This trades having to repeat the computation of encoding for the memory
/// cost of storing an encoded copy
#[derive(Clone)]
pub(crate) struct EncodedCache(Arc<RwLock<Option<Vec<u8>>>>);

impl EncodedCache {
    pub(crate) fn new(encoded: Vec<u8>) -> Self {
        Self(Arc::new(RwLock::new(Some(encoded))))
    }

    pub(crate) fn empty() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }

    pub(crate) fn mark_dirty(&self) {
        self.0.write().take();
    }

    pub(crate) fn get<'a>(&'a self) -> Option<MappedRwLockReadGuard<'a, [u8]>> {
        let cached = self.0.upgradable_read();
        if cached.is_none() {
            return None;
        }
        Some(RwLockReadGuard::map(
            RwLockUpgradableReadGuard::downgrade(cached),
            |inner: &Option<Vec<u8>>| inner.as_deref().unwrap(),
        ))
    }

    pub(crate) fn set<'a>(&'a self, data: Vec<u8>) -> MappedRwLockReadGuard<'a, [u8]> {
        let mut inner = self.0.write();
        let _ = inner.insert(data);
        RwLockReadGuard::map(
            RwLockWriteGuard::downgrade(inner),
            |inner: &Option<Vec<u8>>| inner.as_deref().unwrap(),
        )
    }
}
