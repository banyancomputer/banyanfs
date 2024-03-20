mod create_session_request;
//mod retrieve_request;
mod store_request;

use create_session_request::{CreateSessionRequest, CreateSessionResponse};
//use retrieve_request::RetrieveRequest;
use store_request::{StoreLifecycle, StoreRequest, StoreResponse};

use bytes::Bytes;
use futures::Stream;
use reqwest::Url;

use crate::api::client::{ApiClient, ApiError};

use crate::codec::Cid;

pub async fn create_session(
    client: &ApiClient,
    storage_host_url: &Url,
    metadata_id: &str,
    session_data_size: u64,
) -> Result<CreateSessionResponse, ApiError> {
    let store_request = CreateSessionRequest::new(metadata_id, session_data_size);

    client
        .storage_host_request_full(storage_host_url, store_request)
        .await
}

//pub async fn retrieve(
//    client: &ApiClient,
//    storage_host_url: &Url,
//    cid: &str,
//) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, ApiError> {
//    let response = client
//        .storage_host_request_full(
//            storage_host_url,
//            RetrieveRequest::new(drive_id.into(), metadata_id.into()),
//        )
//        .await?;
//
//    Ok(response.consume().bytes_stream())
//}

pub async fn store_ongoing<S>(
    client: &ApiClient,
    storage_host_url: &Url,
    upload_id: &str,
    cid: &Cid,
    stream_body: S,
) -> Result<StoreResponse, ApiError>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    let lifecycle = StoreLifecycle::Ongoing {
        upload_id: upload_id.into(),
    };

    let store_request = StoreRequest::new(cid.clone(), lifecycle, stream_body).await?;

    client
        .storage_host_request_full(storage_host_url, store_request)
        .await
}

pub async fn store_complete<S>(
    client: &ApiClient,
    storage_host_url: &Url,
    upload_id: &str,
    cid: &Cid,
    stream_body: S,
) -> Result<StoreResponse, ApiError>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    let lifecycle = StoreLifecycle::Complete {
        upload_id: upload_id.into(),
    };

    let store_request = StoreRequest::new(cid.clone(), lifecycle, stream_body).await?;

    client
        .storage_host_request_full(storage_host_url, store_request)
        .await
}
