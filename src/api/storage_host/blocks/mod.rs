use crate::api::client::{ApiClient, ApiError};

use crate::codec::Cid;

pub async fn store(_client: &ApiClient, _cid: &Cid, _data: &[u8]) -> Result<(), ApiError> {
    //client
    //    .storage_host_request_full(StoreRequest::new(cid.into(), data))
    //    .await
    todo!()
}
