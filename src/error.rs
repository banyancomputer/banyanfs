use std::fmt::{self, Display, Formatter};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub struct BanyanFsError(String);

//impl From<&'static str> for BanyanFsError {
//    fn from(val: &'static str) -> Self {
//        Self(val.to_string())
//    }
//}

impl Display for BanyanFsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(feature = "banyan-api")]
impl<E: std::error::Error> From<E> for BanyanFsError {
    fn from(error: E) -> Self {
        Self(error.to_string())
    }
}

//#[cfg(feature = "banyan-api")]
//impl From<serde_json::Error> for BanyanFsError {
//    fn from(error: serde_json::Error) -> Self {
//        Self(error.to_string())
//    }
//}

#[cfg(target_arch = "wasm32")]
impl From<BanyanFsError> for JsValue {
    fn from(error: BanyanFsError) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

//#[cfg(target_arch = "wasm32")]
//impl From<serde_wasm_bindgen::Error> for BanyanFsError {
//    fn from(error: serde_wasm_bindgen::Error) -> Self {
//        Self(error.to_string())
//    }
//}

pub type BanyanFsResult<T> = Result<T, BanyanFsError>;
