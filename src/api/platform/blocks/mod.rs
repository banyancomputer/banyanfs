mod locate_request;

use locate_request::{LocateRequest, LocateResponse};

use crate::api::client::{ApiClient, ApiError};
use crate::codec::Cid;

pub async fn locate(client: &ApiClient, cids: &[Cid]) -> Result<LocateResponse, ApiError> {
    let request = LocateRequest::new(cids.to_vec());
    let resp = client.platform_request_full(request).await?;
    LocateResponse::try_from(resp)
}
