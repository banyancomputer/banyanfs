mod drive_kind;
mod storage_class;

pub use drive_kind::DriveKind;
pub use storage_class::StorageClass;

pub mod drives {
    use serde::{Deserialize, Serialize};

    use crate::api::client::{ApiClient, ApiError, ApiRequest};
    use crate::api::platform::{DriveKind, StorageClass};

    #[derive(Debug, Deserialize, Serialize)]
    #[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
    pub struct ApiDrive {
        pub(crate) id: String,
        pub(crate) name: String,

        #[serde(rename = "type")]
        pub(crate) kind: DriveKind,

        pub(crate) storage_class: StorageClass,
    }

    pub struct GetAllDrivesRequest;

    impl ApiRequest for GetAllDrivesRequest {
        type Response = Vec<ApiDrive>;

        type Payload = ();

        fn path(&self) -> String {
            "/api/v1/buckets".to_string()
        }

        fn payload(&self) -> Option<Self::Payload> {
            None
        }
    }

    pub async fn list_all(client: &ApiClient) -> Result<Vec<ApiDrive>, ApiError> {
        match client.platform_request(GetAllDrivesRequest).await? {
            Some(drives) => Ok(drives),
            None => Err(ApiError::UnexpectedResponse("response should not be empty")),
        }
    }
}
