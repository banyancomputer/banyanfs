mod create_request;
mod get_all_request;
mod get_request;
mod update_request;

use create_request::CreateRequest;
use get_all_request::GetAllRequest;
use get_request::GetRequest;
use update_request::UpdateRequest;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::{ApiDrive, ApiDriveId, DriveKind, StorageClass};
use crate::codec::crypto::VerifyingKey;
use crate::prelude::platform::ApiDriveUpdateAttributes;

pub async fn create(
    client: &ApiClient,
    name: &str,
    owner_key: &VerifyingKey,
) -> Result<ApiDriveId, ApiError> {
    let owner_key_spki = owner_key
        .to_spki()
        .map_err(|e| ApiError::InvalidData(e.to_string()))?;

    let request = CreateRequest {
        name: name.to_string(),
        kind: DriveKind::Interactive,
        storage_class: StorageClass::Hot,
        owner_key: owner_key_spki,
    };

    let created_drive = client.platform_request_full(request).await?;

    Ok(created_drive.id)
}

pub async fn get(client: &ApiClient, drive_id: String) -> Result<ApiDrive, ApiError> {
    let request = GetRequest::new(drive_id);
    let drive = client.platform_request_full(request).await?;
    Ok(drive)
}

pub async fn get_all(client: &ApiClient) -> Result<Vec<ApiDrive>, ApiError> {
    client.platform_request_full(GetAllRequest).await
}

pub async fn update(
    client: &ApiClient,
    drive_id: String,
    attrs: ApiDriveUpdateAttributes,
) -> Result<(), ApiError> {
    let request = UpdateRequest::new(drive_id, attrs);
    client.platform_request_empty_response(request).await
}
