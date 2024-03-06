use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiMetadata {
    id: ApiMetadataId,

    root_cid: RootCid,
    metadata_cid: MetadataCid,
    data_size: u64,
    metadata_size: u64,

    state: ApiMetadataState,

    created_at: i64,
    updated_at: i64,

    snapshot_id: Option<SnapshotId>,
}

impl ApiMetadata {
    pub fn id(&self) -> ApiMetadataId {
        self.id.clone()
    }
}

pub type ApiMetadataId = String;

pub type ApiMetadataState = String;

pub type MetadataCid = String;

pub type RootCid = String;

pub type SnapshotId = String;
