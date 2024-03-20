use async_std::prelude::*;
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::api::client::{ApiError, ApiRequest, StorageHostApiRequest};
use crate::codec::Cid;

pub(crate) enum StoreLifecycle {
    Ongoing { upload_id: String },
    Complete { upload_id: String },
}

pub(crate) struct StoreRequest {
    block_cid: String,
    lifecycle: StoreLifecycle,
    stream_body: Option<Body>,
}

impl StoreRequest {
    pub(crate) async fn new<S>(
        block_cid: Cid,
        lifecycle: StoreLifecycle,
        stream_body: S,
    ) -> Result<Self, std::io::Error>
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
    {
        let block_cid = block_cid.as_base64url_multicodec();

        //#[cfg(target_arch = "wasm32")]
        //let stream_body = if cfg!(target_arch = "wasm32") {
        //    let body_bytes =
        //        crate::api::client::utils::consume_stream_into_bytes(stream_body).await?;
        //    Body::from(body_bytes)
        //} else {
        //    Body::wrap_stream(stream_body)
        //};

        // note(sstelfox): For the client case and general writing cases we need to wrap this in a
        // stream when not targeting WASM, a rough cut was left above. For expendiency I didn't
        // want to diagnose and test both cases so implmented the universal and simpler one.
        let body_bytes = crate::api::client::utils::consume_stream_into_bytes(stream_body).await?;
        let stream_body = Body::from(body_bytes);

        Ok(Self {
            block_cid,
            lifecycle,
            stream_body: Some(stream_body),
        })
    }
}

#[async_trait(?Send)]
impl ApiRequest for StoreRequest {
    type Response = StoreResponse;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        mut request: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        let body = self.stream_body.take().ok_or(ApiError::RequestReused)?;

        let mut form = Form::new();

        let block_details = match &self.lifecycle {
            StoreLifecycle::Ongoing { upload_id } => BlockDetails::Ongoing {
                completed: false,
                upload_id: upload_id.clone(),
            },
            StoreLifecycle::Complete { upload_id } => BlockDetails::Ongoing {
                completed: true,
                upload_id: upload_id.clone(),
            },
        };

        let inner_request = InnerStoreRequest {
            cid: self.block_cid.clone(),
            block_details,
        };

        let json_bytes = serde_json::to_vec(&inner_request)?;
        let json_part = Part::bytes(json_bytes).mime_str("application/json")?;
        form = form.part("request-data", json_part);

        // note: not actually in car format... should adjust this once its been deprecated
        let stream_part = Part::stream(body).mime_str("application/vnd.ipld.car; version=2")?;
        form = form.part("block", stream_part);

        // todo: may need to include a content length header explicitly
        request = request.multipart(form);

        Ok(request)
    }

    fn path(&self) -> String {
        "/api/v1/upload/block".to_string()
    }
}

impl StorageHostApiRequest for StoreRequest {}

#[derive(Serialize)]
struct InnerStoreRequest {
    cid: String,

    #[serde(rename = "details", flatten)]
    block_details: BlockDetails,
}

#[derive(Serialize)]
#[serde(untagged)]
enum BlockDetails {
    Ongoing { completed: bool, upload_id: String },
    OneOff,
}

#[derive(Deserialize)]
pub struct StoreResponse {}
