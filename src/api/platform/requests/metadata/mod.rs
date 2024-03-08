mod get_current_request;
mod pull_request;

use get_current_request::GetCurrentRequest;
use pull_request::PullRequest;

use bytes::Bytes;
use futures::Stream;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiMetadata;

pub async fn get_current(client: &ApiClient, bucket_id: &str) -> Result<ApiMetadata, ApiError> {
    client
        .platform_request_full(GetCurrentRequest::new(bucket_id.into()))
        .await
}

pub async fn pull_stream(
    client: &ApiClient,
    bucket_id: &str,
    metadata_id: &str,
) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, ApiError> {
    let response = client
        .platform_request_full(PullRequest::new(bucket_id.into(), metadata_id.into()))
        .await?;

    Ok(response.consume().bytes_stream())
}
