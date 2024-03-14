mod get_current_request;
mod pull_request;
mod push_request;

use get_current_request::GetCurrentRequest;
use pull_request::PullRequest;
use push_request::{PushRequest, PushResponse};

use bytes::Bytes;
use futures::Stream;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiMetadata;
use crate::codec::crypto::Fingerprint;
use crate::codec::Cid;

pub async fn get_current(client: &ApiClient, drive_id: &str) -> Result<ApiMetadata, ApiError> {
    client
        .platform_request_full(GetCurrentRequest::new(drive_id.into()))
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

pub async fn push_stream<S>(
    client: &ApiClient,
    drive_id: &str,

    expected_data_size: u64,
    merkle_root_cid: Cid,
    previous_merkle_root_cid: Option<String>,

    stream_body: std::pin::Pin<Box<S>>,

    valid_keys: Vec<Fingerprint>,
    deleted_block_cids: Vec<Cid>,
) -> Result<PushResponse, ApiError>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + ?Sized,
{
    let push_request = PushRequest::new(
        drive_id.into(),
        expected_data_size,
        merkle_root_cid,
        previous_merkle_root_cid,
        stream_body,
        valid_keys,
        deleted_block_cids,
    )
    .await?;

    client.platform_request_full(push_request).await
}
