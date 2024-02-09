// Note: wasm-pack test does not run any unit tests that are pub reachable from the root module, I
// think this includes the test module itself

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
    pub use crate::error::*;
    pub use crate::filesystem::*;
    pub use crate::version::*;

    pub use crate::codec::crypto::SigningKey;
    pub use crate::codec::header::FormatHeader;

    pub use crate::codec::AsyncEncodable;
    pub use crate::codec::FilesystemId;
}
