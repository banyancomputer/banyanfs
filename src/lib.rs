#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "banyan-api")]
pub mod api;

pub mod codec;
pub mod error;
pub mod filesystem;
pub mod stores;
pub mod utils;
pub mod version;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Re-export some of our dependencies for QoL, might want to expand this
pub use async_trait;

/// Prelude for the banyanfs library exporting the most commonly used types and traits.
///
/// ```rust
/// use banyanfs::prelude::*;
/// ```
pub mod prelude {
    #[cfg(feature = "banyan-api")]
    pub use crate::api::*;

    pub use crate::error::*;
    pub use crate::filesystem::*;
    pub use crate::stores::*;
    pub use crate::version::*;

    pub use crate::codec::crypto::{SigningKey, VerifyingKey};
    pub use crate::codec::header::ContentOptions;
    pub use crate::codec::FilesystemId;
}
