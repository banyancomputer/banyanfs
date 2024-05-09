mod api_auth;
mod direct_response;
mod error;
mod expiring_token;
mod platform_token;
mod storage_host_auth;
mod traits;

pub use error::ApiClientError;

pub(crate) mod utils;

pub(crate) use api_auth::ApiAuth;
pub(crate) use direct_response::DirectResponse;
pub(crate) use expiring_token::ExpiringToken;
pub(crate) use platform_token::{PlatformToken, PlatformTokenError};
pub(crate) use storage_host_auth::{StorageHostAuth, StorageTokenError};
pub(crate) use traits::{
    ApiRequest, FromReqwestResponse, PlatformApiRequest, StorageHostApiRequest,
};

pub(crate) const PLATFORM_AUDIENCE: &str = "banyan-platform";

pub(crate) const STORAGE_HOST_AUDIENCE: &str = "banyan-storage";

use std::sync::Arc;

use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use tracing::debug;

use crate::codec::crypto::{SigningKey, VerifyingKey};
use crate::prelude::BanyanFsError;

/// An HTTP client for interacting with the Banyan API (both platform and storage hosts). Specific
/// requests can be found the in appropriate module for their request type either
/// [`crate::api::platform`] or [`crate::api::storage_host`].
///
/// The authentication state machine between the platform is fairly complete but documented in the
/// platform API public documentation. This library expects to re-used the key that is protecting
/// and granting access to specific drives to also be a valid key for accessing the APIs.
#[derive(Clone)]
pub struct ApiClient {
    auth: ApiAuth,
    base_url: Url,
    client: Client,
    platform_pubkey: Option<VerifyingKey>,
}

impl ApiClient {
    /// Create a new instance. The base URL is normally either the base HTTP(S) URL of the platform
    /// and will be what is used for all future requets (other than storage hot requests). This
    /// client customizes its user agent so recipients can prevent abuse from requests from this
    /// software if it becomes misconfigured.
    pub fn new(
        base_url: &str,
        account_id: &str,
        key: Arc<SigningKey>,
    ) -> Result<Self, ApiClientError> {
        let base_url = Url::parse(base_url)?;
        let auth = ApiAuth::new(account_id, key);
        let client = default_reqwest_client()?;

        Ok(Self {
            auth,
            base_url,
            client,
            platform_pubkey: None,
        })
    }

    /// Returns the configured base URL for the API client. If you wish to change this you should
    /// create a new [`ApiClient`] instance the desired base URL.
    pub fn base_url(&self) -> Url {
        self.base_url.clone()
    }

    /// Internal method used for making raw requests to any of the Banyan API endpoints, provides
    /// some consistent logging and expects the caller to handle the authentication. In almost all
    /// cases you'll want to use the [`platform_request`] or [`storage_host_request`] methods.
    ///
    /// There are a few places where calling this directly is needed, mostly around authentication
    /// and registration with storage hosts but other exceptions may also exist.
    ///
    /// One thing to be aware of is that this will only ever parse the response from the server as
    /// the [`R::Response`] type if the status code returned from the server was one of the success
    /// codes (2xx).
    pub(crate) async fn request<R: ApiRequest>(
        &self,
        base_url: &Url,
        bearer_token: &str,
        mut request: R,
    ) -> Result<Option<R::Response>, ApiError> {
        debug!(method = %R::METHOD, %base_url, url = %request.path(), "request");

        let full_url = base_url.join(&request.path())?;
        let mut request_builder = self.client.request(R::METHOD, full_url);

        request_builder = request_builder.bearer_auth(bearer_token);
        request_builder = request.add_payload(request_builder).await?;

        let response = request_builder.send().await?;
        let status = response.status();

        debug!(response_status = ?status, "platform_request_response");

        if status.is_success() {
            FromReqwestResponse::from_response(response).await
        } else {
            let resp_bytes = response.bytes().await?;

            if status == StatusCode::UNAUTHORIZED {
                return Err(ApiError::NotAuthorized);
            }

            match serde_json::from_slice::<StandardApiError>(&resp_bytes) {
                Ok(raw_error) => Err(ApiError::Message {
                    status_code: status.as_u16(),
                    message: raw_error.message,
                }),
                Err(_) => {
                    tracing::warn!(response_status = ?status, "api endpoint did not return standard error message");

                    Err(ApiError::Message {
                        status_code: status.as_u16(),
                        message: String::from_utf8_lossy(&resp_bytes).to_string(),
                    })
                }
            }
        }
    }

    /// Perform a request to the platform API. This is more restrictive than the
    /// [`ApiClient::request`] method, limiting the request to only those that are explicitly
    /// implementing the marker trait [`PlatformApiRequest`] but will handle the authentication for
    /// you.
    pub(crate) async fn platform_request<R: PlatformApiRequest>(
        &self,
        request: R,
    ) -> Result<Option<R::Response>, ApiError> {
        // Send authentication if its available even if the request is not marked as requiring it
        let token = self.auth.platform_token().await?;
        self.request(&self.base_url, &token, request).await
    }

    /// When a request to the platform API is expected to return an empty response, this shortcuts
    /// some of the boilerplate and allows enabling a strict mode we can use for validation of or
    /// platform's behavior.
    pub(crate) async fn platform_request_empty_response<R>(
        &self,
        request: R,
    ) -> Result<(), ApiError>
    where
        R: PlatformApiRequest<Response = ()>,
    {
        let resp = self.platform_request(request).await?;

        if cfg!(feature = "strict") && resp.is_some() {
            return Err(ApiError::UnexpectedResponse("expected empty response"));
        }

        Ok(())
    }

    /// This is the most commonly used way to interact with the platform. It should be used
    /// whenever you're expecting a response from the platform in any form. This supports streaming
    /// response as well by using a [`DirectResponse`] in the request's defined response type.
    pub(crate) async fn platform_request_full<R: PlatformApiRequest>(
        &self,
        request: R,
    ) -> Result<R::Response, ApiError> {
        match self.platform_request(request).await? {
            Some(resp) => Ok(resp),
            None => Err(ApiError::UnexpectedResponse("response should not be empty")),
        }
    }

    pub async fn platform_public_key(&mut self) -> Result<VerifyingKey, ApiError> {
        if let Some(pubkey) = &self.platform_pubkey {
            return Ok(pubkey.clone());
        }

        let pubkey = crate::api::platform::status::get_public_key(self).await?;
        self.platform_pubkey = Some(pubkey.clone());
        Ok(pubkey)
    }
    pub async fn record_storage_grant(&self, storage_host_url: Url, auth_token: &str) {
        tracing::debug!(?storage_host_url, "recording storage grant");

        self.auth
            .record_storage_grant(storage_host_url, auth_token)
            .await;
    }

    /// Provides direct access to the internal authentication's signing key that the API client was
    /// initialized with. This isn't really ideal and should be avoided. We'll be refactoring this
    /// out in the future. This isn't a problem but it is a smell that I don't like around
    /// sensitive material.
    pub(crate) fn signing_key(&self) -> Arc<SigningKey> {
        self.auth.signing_key()
    }

    pub(crate) async fn storage_host_request<R: StorageHostApiRequest>(
        &self,
        storage_host_url: &Url,
        request: R,
    ) -> Result<Option<R::Response>, ApiError> {
        let token = self.auth.storage_host_token(self, storage_host_url).await?;

        // todo(sstelfox): add a check on the returned result, if its not authorized we should clear it from
        // the storage auth's authenticated host list

        match self.request(storage_host_url, &token, request).await {
            Ok(resp) => Ok(resp),
            Err(ApiError::NotAuthorized) => {
                self.auth.clear_storage_host_auth(storage_host_url).await;
                Err(ApiError::NotAuthorized)
            }
            Err(e) => Err(e),
        }
    }

    pub(crate) async fn storage_host_request_empty_response<R>(
        &self,
        storage_host_url: &Url,
        request: R,
    ) -> Result<(), ApiError>
    where
        R: StorageHostApiRequest<Response = ()>,
    {
        let resp = self.storage_host_request(storage_host_url, request).await?;

        if cfg!(feature = "strict") && resp.is_some() {
            return Err(ApiError::UnexpectedResponse("expected empty response"));
        }

        Ok(())
    }

    pub(crate) async fn storage_host_request_full<R: StorageHostApiRequest>(
        &self,
        storage_host_url: &Url,
        request: R,
    ) -> Result<R::Response, ApiError> {
        match self.storage_host_request(storage_host_url, request).await? {
            Some(resp) => Ok(resp),
            None => Err(ApiError::UnexpectedResponse("response should not be empty")),
        }
    }
}

fn default_reqwest_client() -> Result<reqwest::Client, ApiClientError> {
    let user_agent = format!("banyanfs/{}", crate::version::minimal_version());
    let client = reqwest::Client::builder().user_agent(user_agent).build()?;

    Ok(client)
}

/// These are API errors that occurs directly as a result of an HTTP request and does not represent
/// a failure in the client or library itself. Please refer to the specific error variant if you're
/// looking for additional diagnostics for addressing the issue.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// The general exception to the rule stated in the error description, these errors are
    /// exepcted to be incredibly rare as they usually result from misuse of the reqwest client.
    /// The way we use the client is fairly standard, though the WASM variant has some heavy
    /// additional restrictions that make these errors more likely to occur due to browser
    /// incompatibilities.
    #[error("network client experienced issue: {0}")]
    ClientError(#[from] reqwest::Error),

    /// The server (could come from either the platform or a storage host) reported that the user
    /// doesn't have enough capacity to complete request. For requests to the platform this is
    /// frequently a result of the account limit and the intended amount of data to store at a
    /// storage host instead of the size of the specific request.
    #[error("the user does not have sufficient authorized capacity to accept the request")]
    InsufficientStorage,

    /// The request that was previously made contained invalid data. This libary's API was designed
    /// to limit the possibility of these kinds of errors by strictly enforcing the types used for
    /// the request parameters. Some parameters still come in as strings and numbers without
    /// supporting all forms of those types.
    ///
    /// Pay attention to the returned error message as it likely contains useful details for
    /// identifying which piece of data specifically was at fault.
    #[error("the data provided to the API client wasn't valid: {0}")]
    InvalidData(String),

    /// The provided URL wasn't valid. This one should be pretty straight-forward as it almost
    /// certainly falls down to an issue with the provided base URL. Issues from generated or
    /// derived URLs should be reported as a bug in the library itself.
    #[error("Request URL is invalid: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// The most common error message returned from the API itself. This captures both the status
    /// code and the specific message that the server returned.
    #[error("API returned {status_code} response with message: {message}")]
    Message { status_code: u16, message: String },

    /// The client will report this error when one of the provided arguments to the call is not
    /// matched against the expectations already set for the client. The message provides more
    /// details, but doing things like attempting to create an API key with the remote API using
    /// the key you're authenticating with will report this kind of an error as the premise of the
    /// request doesn't make sense.
    #[error("response from API did not match our expectations: {0}")]
    MismatchedData(String),

    /// Fairly straight-forward, the request was rejected due to an access control issue. For
    /// storage hosts this may mean that the client simply isn't registered yet and additional
    /// work needs to be done with the platform to authorize the client's key. The internal
    /// mechanisms that handle storage host authentication will handle these cases for you, end
    /// users are only expected to see this in the event the extra authorization steps also fail.
    #[error("the client was not authorized to make the request")]
    NotAuthorized,

    /// While generating a JWT to authenticate a request for the platform, an operation failed.
    /// This is a highly unlikely error as its primarily wrapping operations that shouldn't fail as
    /// long as the arguments are correct. You'll need to refer to the specific error case reported
    /// and the operation for additional context on the failure cause.
    #[error("failed to generate token for platform platform: {0}")]
    PlatformTokenError(#[from] PlatformTokenError),

    /// This is a very specific error case. This library streams its large data uploads by
    /// consuming an asynchronous stream of bytes. When that stream has be consumed by an attempted
    /// upload, and the same request object is attempted to be re-used you'll get this error. The
    /// library caller should create a new request for each upload attempt.
    #[error("unable to reuse streaming requests")]
    RequestReused,

    /// When this error occurs its likely due to a skism between the current client's view of one
    /// of the APIs and what the real API is serving. Fixing this error likely requires either an
    /// update (the client library is outdated) or a pull-request to the library to patch the
    /// affected request/response.
    #[error("failed to during json (de)serialization: {0}")]
    Serde(#[from] serde_json::Error),

    /// Similar to [`Self::PlatformTokenError`] but for storage host tokens. This one is more
    /// common as there is a significantly more complex dance that occurs behind the scene in some
    /// cases. Generation of storage tokens effectively needs to be done online in the general case
    /// as it may need to perform a registration step involving the platform.
    ///
    /// More specific details of the error cases are covered in the [`StorageTokenError`]
    /// definition.
    #[error("failed to generate token for storage host: {0}")]
    StorageTokenError(#[from] StorageTokenError),

    /// When performing some form of streaming I/O an unrecoverable error such as end-of-file in
    /// the stream or in some cases failure to parse the contents of that stream occurred.
    #[error("unexpected I/O error in API client: {0}")]
    StreamingIo(#[from] std::io::Error),

    /// This will occur only when an error was returned from the remote API and the response was
    /// not in the API's standard error format. This is a bug in the API itself and can be reported
    /// through GitHub issues, but we'll almost certainly see this before you do ;)
    #[error("unexpected API response: {0}")]
    UnexpectedResponse(&'static str),

    /// A WASM specific error that likely occurred due to a browser API inconsistency. These are
    /// only present in a few specific operations so the internal cause should reveal more about
    /// the error itself.
    #[cfg(target_arch = "wasm32")]
    #[error("WASM internal error: {0}")]
    WasmInternal(String),
}

impl From<ApiError> for BanyanFsError {
    fn from(error: ApiError) -> Self {
        Self::from(error.to_string())
    }
}

/// This is the inner error type that the API will always return. We don't return this directly as
/// we want to include the status code as well. Will always become a [`ApiError::Message`].
#[derive(Deserialize)]
struct StandardApiError {
    #[serde(rename = "msg")]
    pub message: String,
}
