mod get_current_request;

use get_current_request::GetCurrentRequest;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiMetadata;

pub async fn get_current(client: &ApiClient, bucket_id: &str) -> Result<ApiMetadata, ApiError> {
    client
        .platform_request_full(GetCurrentRequest::new(bucket_id))
        .await
}
