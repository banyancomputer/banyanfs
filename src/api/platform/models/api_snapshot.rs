use serde::{Deserialize, Serialize};

use crate::api::platform::ApiMetadataId;

// note(sstelfox): This api should return a bucket ID as well
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiSnapshot {
    id: ApiSnapshotId,
    metadata_id: ApiMetadataId,

    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<i64>,

    created_at: i64,
}

impl ApiSnapshot {
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn id(&self) -> ApiSnapshotId {
        self.id.clone()
    }

    pub fn metadata_id(&self) -> ApiMetadataId {
        self.metadata_id.clone()
    }

    pub fn size(&self) -> Option<i64> {
        self.size
    }
}

pub type ApiSnapshotId = String;

pub type ApiSnapshotState = String;
