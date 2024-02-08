//#![allow(unused_variables)]
//#![allow(dead_code)]
//#![allow(unused_imports)]

pub mod codec;
pub mod crypto;
pub mod error;
pub mod version;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub mod prelude {
    pub use crate::error::*;
    pub use crate::version::*;
}
