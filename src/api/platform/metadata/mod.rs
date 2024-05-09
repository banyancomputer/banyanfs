mod get_all_request;
mod get_current_request;
mod get_request;
mod pull_request;
mod push_request;

use get_all_request::GetAllRequest;
use get_current_request::GetCurrentRequest;
use get_request::GetRequest;
use pull_request::PullRequest;
use push_request::{PushRequest, PushResponse};

use bytes::Bytes;
use futures::Stream;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::{ApiMetadata, ApiMetadataId};
use crate::codec::crypto::Fingerprint;
use crate::codec::Cid;
use crate::prelude::VerifyingKey;

pub async fn get_current(client: &ApiClient, drive_id: &str) -> Result<ApiMetadata, ApiError> {
    client
        .platform_request_full(GetCurrentRequest::new(drive_id.into()))
        .await
}

pub async fn get(
    client: &ApiClient,
    drive_id: &str,
    metadata_id: &str,
) -> Result<ApiMetadata, ApiError> {
    client
        .platform_request_full(GetRequest::new(drive_id.into(), metadata_id.into()))
        .await
}

pub async fn get_all(client: &ApiClient, drive_id: &str) -> Result<ApiMetadata, ApiError> {
    client
        .platform_request_full(GetAllRequest::new(drive_id.into()))
        .await
}

pub async fn pull_stream(
    client: &ApiClient,
    drive_id: &str,
    metadata_id: &str,
) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, ApiError> {
    let response = client
        .platform_request_full(PullRequest::new(drive_id.into(), metadata_id.into()))
        .await?;

    Ok(response.consume().bytes_stream())
}

#[allow(clippy::too_many_arguments)]
pub async fn push_stream<S>(
    client: &ApiClient,
    drive_id: &str,

    expected_data_size: u64,
    merkle_root_cid: Cid,
    previous_version_id: Option<ApiMetadataId>,

    stream_body: std::pin::Pin<Box<S>>,

    verifying_keys: Vec<VerifyingKey>,
    deleted_block_cids: Vec<Cid>,
) -> Result<PushResponse, ApiError>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + ?Sized,
{
    let push_request = PushRequest::new(
        drive_id.into(),
        expected_data_size,
        merkle_root_cid,
        previous_version_id,
        stream_body,
        verifying_keys,
        deleted_block_cids,
    )
    .await?;

    client.platform_request_full(push_request).await
}
