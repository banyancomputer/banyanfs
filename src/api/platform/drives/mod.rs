mod create_request;
mod delete_request;
mod get_all_request;
mod get_request;
mod update_request;

use create_request::CreateRequest;
use delete_request::DeleteRequest;
use get_all_request::GetAllRequest;
use get_request::GetRequest;
use update_request::UpdateRequest;

use crate::api::client::{ApiClient, ApiError};
use crate::api::platform::{ApiDrive, ApiDriveId, DriveKind, StorageClass};
use crate::codec::crypto::Fingerprint;
use crate::prelude::platform::ApiDriveUpdateAttributes;

pub async fn create(
    client: &ApiClient,
    name: &str,
    fingerprint: &Fingerprint,
) -> Result<ApiDriveId, ApiError> {
    let request = CreateRequest {
        name: name.to_string(),
        kind: DriveKind::Interactive,
        storage_class: StorageClass::Hot,
        fingerprint: fingerprint.as_hex()[..(2 * 20)].to_string(),
    };

    let created_drive = client.platform_request_full(request).await?;

    Ok(created_drive.id)
}

pub async fn delete(client: &ApiClient, drive_id: &str) -> Result<(), ApiError> {
    let request = DeleteRequest::new(drive_id.into());
    client.platform_request_empty_response(request).await
}

pub async fn get(client: &ApiClient, drive_id: &str) -> Result<ApiDrive, ApiError> {
    let request = GetRequest::new(drive_id.into());
    let drive = client.platform_request_full(request).await?;
    Ok(drive)
}

pub async fn get_all(client: &ApiClient) -> Result<Vec<ApiDrive>, ApiError> {
    client.platform_request_full(GetAllRequest).await
}

pub async fn update(
    client: &ApiClient,
    drive_id: &str,
    attrs: ApiDriveUpdateAttributes,
) -> Result<(), ApiError> {
    let request = UpdateRequest::new(drive_id.into(), attrs);
    client.platform_request_empty_response(request).await
}
