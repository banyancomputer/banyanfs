mod current_usage;
mod current_usage_limit;
mod get_storage_grant;
mod register_api_key;

use current_usage::{CurrentUsage, CurrentUsageResponse};
use current_usage_limit::{CurrentUsageLimit, CurrentUsageLimitResponse};
use get_storage_grant::{GetStorageGrant, GetStorageGrantResponse};
use register_api_key::RegisterApiKey;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiDriveKeyId;
use crate::codec::crypto::VerifyingKey;

pub async fn current_usage(client: &ApiClient) -> Result<CurrentUsageResponse, ApiError> {
    client.platform_request_full(CurrentUsage).await
}

pub async fn current_usage_limit(
    client: &ApiClient,
) -> Result<CurrentUsageLimitResponse, ApiError> {
    client.platform_request_full(CurrentUsageLimit).await
}

pub async fn get_storage_grant(
    client: &ApiClient,
    storage_hostname: &str,
) -> Result<GetStorageGrantResponse, ApiError> {
    let get_storage_grant = GetStorageGrant::new(storage_hostname.to_string());
    client.platform_request_full(get_storage_grant).await
}

// note(sstelfox): We don't handle API keys well right now. I think this workflow is a
// little broken. We've captured this for now in ENG-589.
pub async fn register_api_key(
    client: &ApiClient,
    public_key: &VerifyingKey,
) -> Result<ApiDriveKeyId, ApiError> {
    let key_registration = RegisterApiKey::new(public_key);
    let fingerprint = key_registration.fingerprint().to_string();

    let response = client.platform_request_full(key_registration).await?;

    if cfg!(feature = "strict") && response.fingerprint() != fingerprint {
        return Err(ApiError::MismatchedData("fingerprint mismatch".to_string()));
    }

    Ok(response.id().clone())
}
