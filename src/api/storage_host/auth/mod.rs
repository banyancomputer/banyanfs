mod register_grant_request;

use register_grant_request::RegisterGrantRequest;

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
