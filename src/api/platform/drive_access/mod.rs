mod get_all_request;
mod revoke;

use get_all_request::GetAllRequest;
use revoke::RevokeRequest;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiDriveAccess;

pub async fn get_all(client: &ApiClient, drive_id: &str) -> Result<Vec<ApiDriveAccess>, ApiError> {
    client
        .platform_request_full(GetAllRequest::new(drive_id.into()))
        .await
}

pub async fn revoke(client: &ApiClient, drive_id: &str, fingerprint: &str) -> Result<(), ApiError> {
    client
        .platform_request_empty_response(RevokeRequest::new(drive_id.into(), fingerprint.into()))
        .await
}
