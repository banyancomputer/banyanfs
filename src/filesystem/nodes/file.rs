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

    custom_metadata: HashMap<String, String>,

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

        let mut attributes: Vec<Attribute> = Vec::new();

        attributes.push(Attribute::Owner(self.owner));
        attributes.push(Attribute::Permissions(self.permissions));
        attributes.push(Attribute::CreatedAt(self.created_at));
        attributes.push(Attribute::ModifiedAt(self.modified_at));

        for (key, value) in self.custom_metadata.iter() {
            attributes.push(Attribute::Custom {
                key: key.clone(),
                value: value.clone(),
            });
        }

        // Sort lexigraphically by the bytes strings as the RFC specifies
        let mut attribute_bytes = Vec::new();
        for attribute in attributes.into_iter() {
            let mut encoded_attributes = Vec::new();
            attribute.encode(&mut encoded_attributes, 0).await?;
            attribute_bytes.push(encoded_attributes);
        }

        attribute_bytes.sort_unstable();
        for attribute in attribute_bytes.into_iter() {
            cid_content.extend(attribute);
        }

        let hash: [u8; 32] = blake3::hash(cid_content.as_bytes()).into();

        Ok(Cid::from(hash))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("failed to generate cid content: {0}")]
    CidEncodingError(#[from] std::io::Error),
}
