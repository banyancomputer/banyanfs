use std::fmt::{self, Display, Formatter};

/// A common catch all minimal error type for the BanyanFS library. A more specific concrete set of
/// errors will be added in the future now that the primary interfaces and operations have been
/// defined. The specific error text must be referred to identity what failure occurred.
#[derive(Debug)]
pub struct BanyanFsError(pub(crate) String);

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

/// Convenience type for any fallible method that can produce a [`BanyanFsError`].
pub type BanyanFsResult<T> = Result<T, BanyanFsError>;
