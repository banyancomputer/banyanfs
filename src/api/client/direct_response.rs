use async_trait::async_trait;
use reqwest::{Response, StatusCode};

use crate::api::client::{ApiError, FromReqwestResponse};

pub(crate) struct DirectResponse(Response);

impl DirectResponse {
    pub(crate) fn consume(self) -> Response {
        self.0
    }
}

#[async_trait(?Send)]
impl FromReqwestResponse for DirectResponse {
    async fn from_response(response: Response) -> Result<Option<Self>, ApiError> {
        if response.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        } else {
            Ok(Some(Self(response)))
        }
    }
}
