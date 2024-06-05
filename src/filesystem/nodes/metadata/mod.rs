use std::str::FromStr;
mod mime_type;

pub use mime_type::MimeGuesser;

#[derive(Hash, Eq, PartialEq)]
pub enum MetadataKey {
    MimeType,
}

impl MetadataKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetadataKey::MimeType => "mime",
        }
    }

    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            MetadataKey::MimeType => b"mime",
        }
    }

    pub fn from_bytes(key: &[u8]) -> Option<Self> {
        match key {
            b"mime" => Some(MetadataKey::MimeType),
            _ => None,
        }
    }
}

impl FromStr for MetadataKey {
    type Err = winnow::error::ErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mime" => Ok(MetadataKey::MimeType),
            _ => Err(winnow::error::ErrorKind::Token),
        }
    }
}
