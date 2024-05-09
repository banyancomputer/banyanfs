mod create_user_key;
mod current_usage;
mod current_usage_limit;
mod get_storage_grant;
mod rename_user_key;
mod user_key_access;

use create_user_key::CreateUserKey;
use current_usage::{CurrentUsage, CurrentUsageResponse};
use current_usage_limit::{CurrentUsageLimit, CurrentUsageLimitResponse};
use get_storage_grant::{GetStorageGrant, GetStorageGrantResponse};
use rename_user_key::RenameUserKey;
use user_key_access::UserKeyAccess;

use url::Url;

use crate::api::client::utils::api_fingerprint_key;
use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::ApiKeyId;
use crate::codec::crypto::VerifyingKey;

use super::ApiUserKeyAccess;

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
    storage_base_url: Url,
) -> Result<GetStorageGrantResponse, ApiError> {
    let get_storage_grant = GetStorageGrant::new(storage_base_url);
    client.platform_request_full(get_storage_grant).await
}

// note(sstelfox): We don't handle API keys well right now. I think this workflow is a
// little broken. We've captured this for now in ENG-589.
pub async fn create_user_key(
    client: &ApiClient,
    name: &str,
    public_key: &VerifyingKey,
) -> Result<ApiKeyId, ApiError> {
    let fingerprint = api_fingerprint_key(public_key);
    let public_key_pem = public_key
        .to_spki()
        .map_err(|err| ApiError::InvalidData(err.to_string()))?;

    let create_user_key = CreateUserKey::new(name, &public_key_pem);
    let response = client.platform_request_full(create_user_key).await?;

    if cfg!(feature = "strict") && response.fingerprint() != fingerprint {
        return Err(ApiError::MismatchedData("fingerprint mismatch".to_string()));
    }

    Ok(response.id().clone())
}

pub async fn rename_user_key(
    client: &ApiClient,
    name: &str,
    user_key_id: &str,
) -> Result<(), ApiError> {
    let rename_user_key = RenameUserKey::new(name, user_key_id);
    client
        .platform_request_empty_response(rename_user_key)
        .await?;
    Ok(())
}

/// Provide the client with a list of User Keys that should be visible to them
pub async fn user_key_access(client: &ApiClient) -> Result<Vec<ApiUserKeyAccess>, ApiError> {
    client.platform_request_full(UserKeyAccess).await
}
