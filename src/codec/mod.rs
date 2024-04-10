//! # Codec
//!
//! The core of the encoding and decoding of the metadata and block files are contained with this
//! module. Each data structure is broken up into a unique module that composes with others to form
//! the underlying structure of the data. The components have been roughly organized into modules
//! according to their logical groupings but things are still a little hectic and some modules can
//! be found outside their ideal location but we're cleaning things up as we go.

pub mod crypto;
pub mod filesystem;
pub mod header;
pub mod meta;
pub mod parser;
pub mod utils;

pub use meta::*;
pub use parser::*;
