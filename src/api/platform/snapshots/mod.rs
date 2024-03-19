mod create_request;
mod get_all_request;
mod restore_request;

use create_request::CreateRequest;
use get_all_request::GetAllRequest;
use restore_request::RestoreRequest;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::{ApiSnapshot, ApiSnapshotId};
use crate::codec::Cid;

pub async fn create(
    client: &ApiClient,
    bucket_id: &str,
    metadata_id: &str,
    cids: &[Cid],
) -> Result<ApiSnapshotId, ApiError> {
    let resp = client
        .platform_request_full(CreateRequest::new(
            bucket_id.into(),
            metadata_id.into(),
            cids,
        ))
        .await?;

    Ok(resp.snapshot_id())
}

pub async fn get_all(client: &ApiClient, bucket_id: &str) -> Result<Vec<ApiSnapshot>, ApiError> {
    client
        .platform_request_full(GetAllRequest::new(bucket_id.into()))
        .await
}

pub async fn restore(
    client: &ApiClient,
    bucket_id: &str,
    snapshot_id: &str,
) -> Result<(), ApiError> {
    // note(sstelfox): The response isn't useful in this regard, the API should
    // be updated to return a 204
    client
        .platform_request_full(RestoreRequest::new(bucket_id.into(), snapshot_id.into()))
        .await?;

    Ok(())
}
