use async_trait::async_trait;

use crate::api::client::ApiRequest;
use crate::api::platform::ApiDrive;

pub struct GetAllDrivesRequest;

#[async_trait]
impl ApiRequest for GetAllDrivesRequest {
    type Payload = ();
    type Response = Vec<ApiDrive>;

    fn path(&self) -> String {
        "/api/v1/buckets".to_string()
    }
}
