use async_std::sync::RwLock;
use std::sync::Arc;

use crate::codec::meta::Cid;

#[derive(Clone)]
pub(crate) struct CidCache(Arc<RwLock<InnerCidCache>>);

impl CidCache {
    pub(crate) async fn cid(&self) -> Result<Cid, CidCacheError> {
        let inner = self.0.read().await;
        inner.cid.clone().ok_or(CidCacheError::CidNotAvailable)
    }

    pub(crate) fn empty() -> Self {
        Self(Arc::new(RwLock::new(InnerCidCache {
            dirty: true,
            cid: None,
            encoded: None,
        })))
    }

    pub(crate) async fn is_dirty(&self) -> bool {
        let inner = self.0.read().await;
        inner.dirty
    }

    pub(crate) async fn mark_dirty(&self) {
        let mut inner = self.0.write().await;
        inner.dirty = true;

        // Invalidate our caches as well
        inner.cid = None;
        inner.encoded = None;
    }

    pub(crate) async fn set_for_cache(&self, data: Vec<u8>) {
        self.set_with_ref(data.as_slice()).await;
        let mut inner = self.0.write().await;
        inner.encoded = Some(data);
    }

    pub(crate) async fn set_with_ref(&self, data: &[u8]) {
        let hash: [u8; 32] = blake3::hash(&data).into();
        let cid = Cid::from(hash);

        let mut inner = self.0.write().await;
        inner.cid = Some(cid);
    }

    pub(crate) async fn take_cached(&self) -> Option<Vec<u8>> {
        let mut inner = self.0.write().await;
        inner.encoded.take()
    }
}

struct InnerCidCache {
    dirty: bool,
    cid: Option<Cid>,
    encoded: Option<Vec<u8>>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CidCacheError {
    #[error("data hasn't been provided to calculate a CID yet")]
    CidNotAvailable,
}
