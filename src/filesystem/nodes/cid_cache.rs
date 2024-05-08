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

    // # Note
    //
    // This function intentionally is not calculating and setting the CID here. If the data
    // provided is actually from this associated node, then when it was encoded it would have
    // calculated its own CID with the [`set_with_ref`] function. This just allows us to use the
    // CID generated encoding for when we come back through and are ready for the whole thing.
    pub(crate) async fn set_cached(&self, data: Vec<u8>) {
        let mut inner = self.0.write().await;
        inner.encoded = Some(data);
    }

    pub(crate) async fn set_with_ref(&self, data: &[u8]) {
        let cid = crate::utils::calculate_cid(data);
        let mut inner = self.0.write().await;
        inner.cid = Some(cid);
        inner.dirty = false;
    }

    pub(crate) async fn take_cached(&self) -> Option<Vec<u8>> {
        let mut inner = self.0.write().await;
        inner.encoded.take()
    }
}

impl From<Cid> for CidCache {
    fn from(cid: Cid) -> Self {
        let inner = InnerCidCache {
            dirty: false,
            cid: Some(cid),
            encoded: None,
        };

        Self(Arc::new(RwLock::new(inner)))
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
