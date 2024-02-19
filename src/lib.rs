// Note: wasm-pack test does not run any unit tests that are pub reachable from the root module, I
// think this includes the test module itself

#[cfg(feature = "banyan-api")]
pub mod api;

pub mod codec;
pub mod error;
pub mod filesystem;
pub mod utils;
pub mod version;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Re-export some of our dependencies for QoL, might want to expand this
pub use async_trait;

pub mod prelude {
    #[cfg(feature = "banyan-api")]
    pub use crate::api::*;

    pub use crate::error::*;
    pub use crate::filesystem::*;
    pub use crate::version::*;

    pub use crate::codec::crypto::{SigningKey, VerifyingKey};
    pub use crate::codec::{AsyncEncodable, FilesystemId};
}
