mod get_all_request;

use get_all_request::GetAllRequest;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiUserKeyAccess;

pub async fn get_all(
    client: &ApiClient,
    drive_id: &str,
) -> Result<Vec<ApiUserKeyAccess>, ApiError> {
    client
        .platform_request_full(GetAllRequest::new(drive_id.into()))
        .await
}
