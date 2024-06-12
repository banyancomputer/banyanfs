use std::str::FromStr;
#[cfg(feature = "mime-type")]
mod mime_type;

#[cfg(feature = "mime-type")]
pub use mime_type::MimeGuesser;

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum MetadataKey {
    MimeType,
    Custom(String),
}

impl MetadataKey {
    pub fn as_str(&self) -> &str {
        match self {
            MetadataKey::MimeType => "mime",
            MetadataKey::Custom(s) => s.as_str(),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            MetadataKey::MimeType => b"mime".to_vec(),
            MetadataKey::Custom(s) => s.as_bytes().to_vec(),
        }
    }

    pub fn from_bytes(key: &[u8]) -> Option<Self> {
        match key {
            b"mime" => Some(MetadataKey::MimeType),
            _ => {
                if key.len() > 255 {
                    return None;
                }

                match std::str::from_utf8(key) {
                    Ok(s) => Some(MetadataKey::Custom(s.to_string())),
                    Err(_) => None,
                }
            }
        }
    }
}

impl FromStr for MetadataKey {
    type Err = winnow::error::ErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mime" => Ok(MetadataKey::MimeType),
            _ => {
                if s.len() > 255 {
                    return Err(winnow::error::ErrorKind::Verify);
                }

                Ok(MetadataKey::Custom(s.to_string()))
            }
        }
    }
}
