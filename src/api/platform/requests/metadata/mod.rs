mod pull_current_request;

use pull_current_request::PullCurrentRequest;

use crate::api::client::{ApiClient, ApiError};

pub async fn pull_current(_client: &ApiClient, bucket_id: &str) -> Result<Vec<u8>, ApiError> {
    let _request = PullCurrentRequest::new(bucket_id);
    todo!("pull actual metadata, needs to support streaming and multipart")
}
