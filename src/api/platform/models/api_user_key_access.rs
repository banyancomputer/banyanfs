use super::ApiUserKey;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ApiUserKeyAccess {
    pub key: ApiUserKey,
    pub bucket_ids: Vec<String>,
}
