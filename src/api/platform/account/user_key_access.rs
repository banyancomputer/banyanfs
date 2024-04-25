use async_trait::async_trait;
use serde::Deserialize;

use crate::api::{
    client::{ApiRequest, PlatformApiRequest},
    platform::ApiUserKeyAccess,
};

pub(crate) struct ListUserKeyAccess;

type BucketId = String;
#[derive(Deserialize)]
pub struct ListUserKeyAccessResponse {
    key_access: Vec<ApiUserKeyAccess>,
}

#[async_trait]
impl ApiRequest for ListUserKeyAccess {
    type Response = ListUserKeyAccessResponse;
    fn path(&self) -> String {
        "/api/v1/auth/user_key_access".to_string()
    }
}

impl PlatformApiRequest for ListUserKeyAccess {}
