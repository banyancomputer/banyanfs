use async_trait::async_trait;

use crate::api::{
    client::{ApiRequest, PlatformApiRequest},
    platform::ApiUserKeyAccess,
};

pub(crate) struct UserKeyAccess;

#[async_trait]
impl ApiRequest for UserKeyAccess {
    type Response = Vec<ApiUserKeyAccess>;
    fn path(&self) -> String {
        "/api/v1/auth/user_key_access".to_string()
    }
}

impl PlatformApiRequest for UserKeyAccess {}
