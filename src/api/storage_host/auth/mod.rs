//! The storage host authentication is a bit more complex to allow for future expansion into more
//! of a decentralized setup and to remove the need for the central platform to need any awareness
//! of access to the privately controlled storage hosts besides providing initial registration.
//!
//! The endpoints supported in this module are used for that initial registration process
//! afterwhich the storage hosts will be registered and can interact with them using their JWTs
//! they can mint themselves. These endpoints will be used again if the client needs additional
//! storage with a particular storage host (which is tracked and authorized by the platform through
//! storage grants).

mod register_grant_request;
mod who_am_i_request;

use register_grant_request::RegisterGrantRequest;
use who_am_i_request::{WhoAmIRequest, WhoAmIResponse};

use reqwest::Url;

use crate::api::client::{ApiClient, ApiError};

/// Storage grants are used to both register a client's public key as an authorized client, and
/// authorizes a certain amount of storage space to be used by the client. The grant itself is
/// produced by the platform to authorized users and must match the public key of the client that
/// is attempting to call this registration endpoint.
///
/// Internally this very specifically uses the raw [`ApiClient::request`] method as it needs to
/// avoid our normal authentication to storage hosts as the grant itself takes that role.
pub async fn register_grant(
    client: &ApiClient,
    storage_host_url: &Url,
    grant_token: &str,
) -> Result<(), ApiError> {
    let verifying_key = client.signing_key().verifying_key();
    let request = RegisterGrantRequest::new(verifying_key);

    let resp = client
        .request(storage_host_url, grant_token, request)
        .await?;

    if cfg!(feature = "strict") && resp.is_some() {
        return Err(ApiError::UnexpectedResponse("expected empty response"));
    }

    Ok(())
}

/// Calls to this endpoint allow a client to sign a JWT and check if its already registered and
/// known to a particular storage host. This is internally used as part of the token registration
/// process and as such needs to avoid the normal storage host authentication workflow. To avoid
/// the normal authentication, the [`ApiClient::request`] method is used directly.
pub async fn who_am_i(
    client: &ApiClient,
    storage_host_url: &Url,
    grant_token: &str,
) -> Result<WhoAmIResponse, ApiError> {
    let response = client
        .request(storage_host_url, grant_token, WhoAmIRequest)
        .await?;

    match response {
        Some(resp) => Ok(resp),
        None => Err(ApiError::UnexpectedResponse("response should not be empty")),
    }
}
