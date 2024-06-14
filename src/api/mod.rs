//! This module contains an implementation of API clients for the Banyan platform and storage host
//! APIs. The clients accept custom base URLs so could be re-implemented to support other storage
//! systems if the implementors so desired.
//!
//! Ongoing use and support is only guaranteed to work with the Banyan storage platform.

pub mod platform;
pub mod storage_host;

pub(crate) mod client;

pub use client::{ApiClient, ApiClientError, ApiError, VecStream};
