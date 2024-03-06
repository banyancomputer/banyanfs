use async_trait::async_trait;
use reqwest::{Response, StatusCode};

use crate::api::client::{ApiError, FromReqwestResponse};

pub(crate) struct DirectResponse(pub(crate) reqwest::Response);

#[async_trait]
impl FromReqwestResponse for DirectResponse {
    async fn from_response(response: Response) -> Result<Option<Self>, ApiError> {
        if response.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        } else {
            Ok(Some(Self(response)))
        }
    }
}
