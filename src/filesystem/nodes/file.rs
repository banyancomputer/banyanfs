use std::collections::HashMap;

use nom::AsBytes;
use time::OffsetDateTime;

use crate::codec::filesystem::{Attribute, Permissions};
use crate::codec::{ActorId, AsyncEncodable, Cid};
use crate::filesystem::ContentReference;

pub struct File {
    owner: ActorId,

    permissions: Permissions,
    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,

    metadata: HashMap<String, String>,

    content: Vec<ContentReference>,
}

impl File {
    pub async fn calculate_cid(&self) -> Result<Cid, FileError> {
        let mut cid_content = Vec::new();

        for content in self.content.iter() {
            content
                .encode(&mut cid_content, 0)
                .await
                .map_err(FileError::CidEncodingError)?;
        }

        let mut attributes = vec![
            Attribute::Owner(self.owner()),
            Attribute::Permissions(self.permissions()),
            Attribute::CreatedAt(self.created_at()),
            Attribute::ModifiedAt(self.modified_at()),
        ];

        for (key, value) in self.metadata.iter() {
            match key.as_str() {
                // This is an optional named attribute that lives in the metadata and can be
                // encoded a little more efficiently
                "mime_type" => attributes.push(Attribute::MimeType(value.clone())),
                _ => {
                    attributes.push(Attribute::Custom {
                        key: key.clone(),
                        value: value.clone(),
                    });
                }
            }
        }

        // Encode our attributes into their final byte strings. We wait here to add them into the
        // overall encoding as they may be in any order at this point
        let mut attribute_bytes = Vec::new();
        for attribute in attributes.into_iter() {
            let mut encoded_attributes = Vec::new();
            attribute.encode(&mut encoded_attributes, 0).await?;
            attribute_bytes.push(encoded_attributes);
        }

        // Sort lexigraphically by the bytes strings as the RFC specifies
        attribute_bytes.sort_unstable();
        for attribute in attribute_bytes.into_iter() {
            cid_content.extend(attribute);
        }

        let hash: [u8; 32] = blake3::hash(cid_content.as_bytes()).into();

        Ok(Cid::from(hash))
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn modified_at(&self) -> OffsetDateTime {
        self.modified_at
    }

    pub fn owner(&self) -> ActorId {
        self.owner
    }

    pub fn permissions(&self) -> Permissions {
        self.permissions
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("failed to generate cid content: {0}")]
    CidEncodingError(#[from] std::io::Error),
}
