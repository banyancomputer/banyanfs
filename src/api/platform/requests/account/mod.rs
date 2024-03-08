mod current_usage;
mod current_usage_limit;
//mod register_api_key;

use current_usage::{CurrentUsage, CurrentUsageResponse};
use current_usage_limit::{CurrentUsageLimit, CurrentUsageLimitResponse};
//use register_api_key::RegisterApiKey;

use crate::api::client::{ApiClient, ApiError};
//use crate::api::platform::ApiDriveKeyId;
//use crate::codec::crypto::VerifyingKey;

pub async fn current_usage(client: &ApiClient) -> Result<CurrentUsageResponse, ApiError> {
    client.platform_request_full(CurrentUsage).await
}

pub async fn current_usage_limit(
    client: &ApiClient,
) -> Result<CurrentUsageLimitResponse, ApiError> {
    client.platform_request_full(CurrentUsageLimit).await
}

// note(sstelfox): I intentionally removed this, we don't handle API keys well right now and I
// think this workflow is inherentily broken. It needs to be rethought a little bit and
// implementing this would be a mistake.
//pub async fn register_api_key(
//    client: &ApiClient,
//    public_key: &VerifyingKey,
//) -> Result<ApiDriveKeyId, ApiError> {
//    let key_registration = RegisterApiKey::new(public_key);
//    let fingerprint = key_registration.fingerprint().to_string();
//
//    let response = client.platform_request_full(key_registration).await?;
//
//    if cfg!(feature = "strict") && response.fingerprint() != fingerprint {
//        return Err(ApiError::MismatchedData("fingerprint mismatch".to_string()));
//    }
//
//    Ok(response.id().clone())
//}
