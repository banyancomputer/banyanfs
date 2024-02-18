pub mod drives {
    use serde::{Deserialize, Serialize};

    use crate::api::client::{ApiClient, ApiError, ApiRequest};

    #[derive(Debug, Deserialize, Serialize)]
    #[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
    pub struct ApiDrive {
        pub(crate) id: String,
        pub(crate) name: String,

        #[serde(rename = "type")]
        pub(crate) kind: String,

        pub(crate) storage_class: String,
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
