mod current_usage;

use current_usage::CurrentUsage;

use crate::api::client::{ApiClient, ApiError};

pub async fn current_usage(client: &ApiClient) -> Result<u64, ApiError> {
    let account_usage = client.platform_request_with_response(CurrentUsage).await?;

    todo!()
}
