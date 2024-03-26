mod register_grant_request;
mod who_am_i_request;

use register_grant_request::RegisterGrantRequest;
use who_am_i_request::{WhoAmIRequest, WhoAmIResponse};

use reqwest::Url;

use crate::api::client::{ApiClient, ApiError};

pub async fn register_grant(
    client: &ApiClient,
    storage_host_url: &Url,
    grant_token: &str,
) -> Result<(), ApiError> {
    let verifying_key = client
        .signing_key()
        .ok_or(ApiError::RequiresAuth)?
        .verifying_key();

    let request = RegisterGrantRequest::new(verifying_key);

    client
        .request(storage_host_url, Some(grant_token.to_string()), request)
        .await?;

    Ok(())
}

pub async fn who_am_i(
    client: &ApiClient,
    storage_host_url: &Url,
) -> Result<WhoAmIResponse, ApiError> {
    client
        .storage_host_request_full(storage_host_url, WhoAmIRequest)
        .await
}
