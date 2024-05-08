mod get_public_key;

use get_public_key::GetPublicKey;

use crate::api::client::{ApiClient, ApiError};
use crate::codec::crypto::VerifyingKey;

pub async fn get_public_key(client: &ApiClient) -> Result<VerifyingKey, ApiError> {
    let response = client.platform_request_full(GetPublicKey).await?;

    let public_key = VerifyingKey::from_spki(response.public_key())
        .map_err(|err| ApiError::MismatchedData(err.to_string()))?;

    Ok(public_key)
}
