use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ApiMetadata {
    pub id: ApiMetadataId,

    pub root_cid: String,
    pub metadata_cid: String,
    pub data_size: u64,
    pub metadata_size: u64,

    pub state: ApiMetadataState,

    pub created_at: i64,
    pub updated_at: i64,

    pub snapshot_id: Option<String>,
}

pub type ApiMetadataId = String;

pub type ApiMetadataState = String;
