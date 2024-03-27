mod register_grant_request;
mod who_am_i_request;

use register_grant_request::RegisterGrantRequest;
use who_am_i_request::{WhoAmIRequest, WhoAmIResponse};

use reqwest::Url;

use crate::api::client::{ApiClient, ApiError};

/// Care is warranted when making changes to these functions, as they need to use the underlying
/// client's raw request method. The convenient wrapper methods perform authentication checks using
/// these, switching to them would cause trigger an async recursion error which should not be
/// worked around by boxing these futures.

pub async fn register_grant(
    client: &ApiClient,
    storage_host_url: &Url,
    grant_token: &str,
) -> Result<(), ApiError> {
    let verifying_key = client.signing_key().verifying_key();
    let request = RegisterGrantRequest::new(verifying_key);

    client
        .request(storage_host_url, grant_token, request)
        .await?;

    Ok(())
}

pub async fn who_am_i(
    client: &ApiClient,
    storage_host_url: &Url,
    grant_token: &str,
) -> Result<WhoAmIResponse, ApiError> {
    let response = client
        .request(storage_host_url, grant_token, WhoAmIRequest)
        .await?;

    match response {
        Some(resp) => Ok(resp),
        None => Err(ApiError::UnexpectedResponse("response should not be empty")),
    }
}
