use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::requests::{CreateDriveRequest, GetAllDrivesRequest};
use crate::api::platform::{ApiDrive, DriveId, DriveKind, StorageClass};
use crate::codec::crypto::VerifyingKey;

pub async fn create(
    client: &ApiClient,
    name: &str,
    owner_key: &VerifyingKey,
) -> Result<DriveId, ApiError> {
    let owner_key_spki = owner_key
        .to_spki()
        .map_err(|e| ApiError::InvalidData(e.to_string()))?;

    let request = CreateDriveRequest {
        name: name.to_string(),
        kind: DriveKind::Interactive,
        storage_class: StorageClass::Hot,
        owner_key: owner_key_spki,
    };

    let created_drive = client.platform_request_with_response(request).await?;

    Ok(created_drive.id)
}

pub async fn list_all(client: &ApiClient) -> Result<Vec<ApiDrive>, ApiError> {
    let drives = client
        .platform_request_with_response(GetAllDrivesRequest)
        .await?;

    Ok(drives)
}
