mod register_grant_request;

use register_grant_request::RegisterGrantRequest;

use crate::api::client::{ApiClient, ApiError};

pub async fn register_grant(client: &ApiClient, grant_token: &str) -> Result<(), ApiError> {
    let verifying_key = client
        .signing_key()
        .ok_or(ApiError::RequiresAuth)?
        .verifying_key();

    let request = RegisterGrantRequest::new(verifying_key, grant_token.into());
    client.storage_host_request_full(request).await?;

    Ok(())
}
