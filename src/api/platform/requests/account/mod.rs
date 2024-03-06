mod current_usage;
mod current_usage_limit;

use current_usage::{CurrentUsage, CurrentUsageResponse};
use current_usage_limit::{CurrentUsageLimit, CurrentUsageLimitResponse};

use crate::api::client::{ApiClient, ApiError};

pub async fn current_usage(client: &ApiClient) -> Result<CurrentUsageResponse, ApiError> {
    client.full_platform_request(CurrentUsage).await
}

pub async fn current_usage_limit(
    client: &ApiClient,
) -> Result<CurrentUsageLimitResponse, ApiError> {
    client.full_platform_request(CurrentUsageLimit).await
}
