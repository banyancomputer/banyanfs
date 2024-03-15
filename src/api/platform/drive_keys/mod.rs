mod get_all_request;

use get_all_request::GetAllRequest;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiDriveKey;

pub async fn get_all(client: &ApiClient, bucket_id: &str) -> Result<Vec<ApiDriveKey>, ApiError> {
    client
        .platform_request_full(GetAllRequest::new(bucket_id.into()))
        .await
}
