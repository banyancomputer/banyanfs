use async_std::prelude::*;
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Method, RequestBuilder, Url};
use serde::{Deserialize, Serialize};

use crate::api::client::utils::api_fingerprint_key;
use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::api::platform::{ApiDriveId, ApiMetadataId};

use crate::codec::Cid;
use crate::prelude::VerifyingKey;

pub(crate) struct PushRequest {
    drive_id: ApiDriveId,

    expected_data_size: u64,

    // formerly just root_cid, doubles up as metadata_cid now
    merkle_root_cid: Cid,
    previous_version_id: Option<ApiMetadataId>,

    stream_body: Option<Body>,

    // note(sstelfox): The following attributes are superceded by the format itself
    // and will no longer be needed but need to be passed in from the outside until
    // the server side has been updated to consume this information from the format
    // itself.
    verifying_keys: Vec<VerifyingKey>,
    deleted_block_cids: Vec<Cid>,
}

impl PushRequest {
    pub(crate) async fn new<S>(
        drive_id: ApiDriveId,

        expected_data_size: u64,
        merkle_root_cid: Cid,

        previous_version_id: Option<ApiMetadataId>,

        stream_body: S,

        verifying_keys: Vec<VerifyingKey>,
        deleted_block_cids: Vec<Cid>,
    ) -> Result<Self, std::io::Error>
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
    {
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
            drive_id,

            expected_data_size,
            merkle_root_cid,
            previous_version_id,

            stream_body: Some(stream_body),

            verifying_keys,
            deleted_block_cids,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PushKey {
    fingerprint: String,
    public_key: String,
}

#[async_trait(?Send)]
impl ApiRequest for PushRequest {
    type Response = PushResponse;

    const METHOD: Method = Method::POST;

    async fn add_payload(
        &mut self,
        mut request: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        let root_cid = self.merkle_root_cid.as_base64url_multicodec();
        let mut previous_id = None;

        // We can drop the multipart uploads and have a simpler upload by leveraging the headers to
        // send the extra data we need.
        request = request
            .header("x-expected-data-size", self.expected_data_size.to_string())
            .header("x-merkle-root-cid", root_cid.clone());

        if let Some(prev_version_id) = &self.previous_version_id {
            request = request.header("x-previous-version-id", prev_version_id.clone());
            previous_id = Some(prev_version_id.clone())
        }

        let body = self.stream_body.take().ok_or(ApiError::RequestReused)?;

        // note: we do not include the attributes we intend to deprecate. In the future, the
        // multipart here will can be replaced simply with the following:
        // request = request.body(body);

        let mut form = Form::new();

        let metadata_cid = root_cid.clone();
        let user_keys = self
            .verifying_keys
            .iter()
            .filter_map(|key| {
                if let Ok(public_key) = key.to_spki() {
                    Some(PushKey {
                        fingerprint: api_fingerprint_key(key),
                        public_key,
                    })
                } else {
                    None
                }
            })
            .collect();
        let deleted_block_cids = self
            .deleted_block_cids
            .iter()
            .map(|c| c.as_base64url_multicodec())
            .collect();

        let inner_request = InnerPushRequest {
            expected_data_size: self.expected_data_size,

            root_cid,
            metadata_cid,
            previous_id,

            user_keys,
            deleted_block_cids,
        };

        let json_bytes = serde_json::to_vec(&inner_request)?;
        let json_part = Part::bytes(json_bytes).mime_str("application/json")?;
        form = form.part("request-data", json_part);

        // note: not actually in car format... should adjust this once its been deprecated
        let stream_part = Part::stream(body).mime_str("application/vnd.ipld.car; version=2")?;
        form = form.part("car-upload", stream_part);

        request = request.multipart(form);

        Ok(request)
    }

    fn path(&self) -> String {
        format!("/api/v1/buckets/{}/metadata", self.drive_id,)
    }
}

impl PlatformApiRequest for PushRequest {}

#[derive(Debug, Serialize)]
struct InnerPushRequest {
    expected_data_size: u64,

    root_cid: String,
    metadata_cid: String,

    previous_id: Option<String>,

    user_keys: Vec<PushKey>,
    deleted_block_cids: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PushResponse {
    id: ApiMetadataId,
    state: String,

    storage_host: Option<String>,
    storage_authorization: Option<String>,
}

#[allow(dead_code)]
impl PushResponse {
    pub(crate) fn id(&self) -> ApiMetadataId {
        self.id.clone()
    }

    pub(crate) fn state(&self) -> &str {
        &self.state
    }

    pub(crate) fn storage_authorization(&self) -> Option<&str> {
        self.storage_authorization.as_deref()
    }

    pub(crate) fn storage_host(&self) -> Option<Url> {
        self.storage_host
            .as_deref()
            .and_then(|s| Url::parse(s).ok())
    }
}
