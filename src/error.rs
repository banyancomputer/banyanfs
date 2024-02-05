use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub struct BanyanFsError(pub &'static str);

impl From<&'static str> for BanyanFsError {
    fn from(val: &'static str) -> Self {
        Self(val)
    }
}

impl Display for BanyanFsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for BanyanFsError {}

#[cfg(target_arch = "wasm32")]
impl From<BanyanFsError> for JsValue {
    fn from(error: BanyanFsError) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

pub type BanyanFsResult<T> = Result<T, BanyanFsError>;
