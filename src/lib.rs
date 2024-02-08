#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]

// Note: wasm-pack test does not run any unit tests that are pub reachable from the root module, I
// think this includes the test module itself

pub mod codec;
pub mod error;
pub mod filesystem;
pub mod utils;
pub mod version;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {
    pub use crate::error::*;
    pub use crate::filesystem::*;
    pub use crate::version::*;

    pub use crate::codec::crypto::SigningKey;
    pub use crate::codec::header::FormatHeader;
    pub use crate::codec::AsyncEncodable;
}
