use std::fmt::{self, Display, Formatter};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub struct BanyanFsError(String);

impl From<&'static str> for BanyanFsError {
    fn from(val: &'static str) -> Self {
        Self(val.to_string())
    }
}

impl From<String> for BanyanFsError {
    fn from(val: String) -> Self {
        Self(val)
    }
}

impl Display for BanyanFsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(feature = "banyan-api")]
impl From<serde_json::Error> for BanyanFsError {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}

pub type BanyanFsResult<T> = Result<T, BanyanFsError>;
