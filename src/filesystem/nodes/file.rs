use std::collections::HashMap;

use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::le_u8;
use time::OffsetDateTime;

use crate::codec::filesystem::{Attribute, Permissions};
use crate::codec::{ActorId, AsyncEncodable, Cid};
use crate::filesystem::FileContent;

pub struct File {
    owner: ActorId,

    permissions: Permissions,
    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,

    metadata: HashMap<String, String>,

    content: FileContent,
}

impl File {
    pub async fn calculate_cid(&self) -> Result<Cid, FileError> {
        let mut cid_content = Vec::new();
        self.encode(&mut cid_content, 0).await?;
        let hash: [u8; 32] = blake3::hash(&cid_content).into();

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

    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, attribute_count) = le_u8(input)?;

        let mut owner = None;
        let mut permissions = None;
        let mut created_at = None;
        let mut modified_at = None;
        let mut metadata = HashMap::new();

        let (remaining, attributes) = Attribute::parse_many(input, attribute_count)?;

        // Validate that we have all the required attributes
        let owner = owner.ok_or(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;
        let permissions =
            permissions.ok_or(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;
        let created_at =
            created_at.ok_or(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;
        let modified_at =
            modified_at.ok_or(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)))?;

        let (remaining, content) = FileContent::parse(remaining)?;

        let file = Self {
            owner,
            permissions,
            created_at,
            modified_at,
            metadata,
            content,
        };

        Ok((remaining, file))
    }

    pub fn permissions(&self) -> Permissions {
        self.permissions
    }
}

#[async_trait]
impl AsyncEncodable for File {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        mut pos: usize,
    ) -> std::io::Result<usize> {
        let attribute_count = 4 + self.metadata.len();
        if attribute_count > 255 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "metadata has too many entries to encode in the file",
            ));
        }

        writer.write_all(&[attribute_count as u8]).await?;
        pos += 1;

        // We know we need to order everything based on the byte, but since these have reserved
        // types we know they'll sort before any of the other attribtues. We can take a shortcut
        // and just encode themn directly in the order we know they'll appear.
        pos = Attribute::Owner(self.owner()).encode(writer, pos).await?;
        pos = Attribute::Permissions(self.permissions())
            .encode(writer, pos)
            .await?;
        pos = Attribute::CreatedAt(self.created_at())
            .encode(writer, pos)
            .await?;
        pos = Attribute::ModifiedAt(self.modified_at())
            .encode(writer, pos)
            .await?;

        let mut attributes = Vec::new();
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
            writer.write_all(&attribute).await?;
            pos += attribute.len();
        }

        pos = self.content.encode(writer, pos).await?;

        Ok(pos)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("failed to generate cid content: {0}")]
    CidEncodingError(#[from] std::io::Error),
}
