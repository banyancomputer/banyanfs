mod get_all_snapshots_request;

use get_all_snapshots_request::GetAllSnapshotsRequest;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiSnapshot;

pub async fn get_all(client: &ApiClient, bucket_id: &str) -> Result<Vec<ApiSnapshot>, ApiError> {
    client
        .platform_request_full(GetAllSnapshotsRequest::new(bucket_id.into()))
        .await
}
