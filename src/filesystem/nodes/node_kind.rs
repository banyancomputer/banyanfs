use std::collections::HashMap;

use crate::codec::filesystem::{DirectoryPermissions, FilePermissions};
use crate::codec::meta::PermanentId;
use crate::filesystem::FileContent;

pub enum NodeKind {
    File {
        permissions: FilePermissions,
        content: FileContent,
    },
    Directory {
        permissions: DirectoryPermissions,
        children: HashMap<String, PermanentId>,
        children_size: u64,
    },
}

impl NodeKind {
    pub fn new_directory() -> Self {
        NodeKind::Directory {
            permissions: DirectoryPermissions::default(),
            children: HashMap::new(),
            children_size: 0,
        }
    }

    pub fn stub_file(size: u64) -> Self {
        NodeKind::File {
            permissions: FilePermissions::default(),
            content: FileContent::Stub { size },
        }
    }
}

//use crate::codec::filesystem::{Attribute, FilePermissions};
//use crate::codec::{ActorId, AsyncEncodable, Cid};
//use std::collections::HashMap;
//
//use async_trait::async_trait;
//use futures::{AsyncWrite, AsyncWriteExt};
//use nom::bytes::streaming::take;
//use nom::error::Error as NomError;
//use nom::error::ErrorKind;
//use nom::number::streaming::le_u8;
//use time::OffsetDateTime;
//
//
//const MIME_TYPE_KEY: &str = "mime_type";
//
//#[derive(Clone, Debug)]
//pub struct File {
//}
//
//impl File {
//pub async fn calculate_cid(&self) -> Result<Cid, FileError> {
//    let mut cid_content = Vec::new();
//    self.encode(&mut cid_content).await?;
//    let hash: [u8; 32] = blake3::hash(&cid_content).into();
//    Ok(Cid::from(hash))
//}

//pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
//    let (remaining, id_bytes) = take(16u8)(input)?;

//    let mut id = [0u8; 16];
//    id.copy_from_slice(id_bytes);

//    let (remaining, attribute_count) = le_u8(remaining)?;

//    let mut owner = None;
//    let mut permissions = None;
//    let mut created_at = None;
//    let mut modified_at = None;
//    let mut metadata = HashMap::new();

//    let (remaining, attributes) = Attribute::parse_many(remaining, attribute_count)?;

//    for attribute in attributes.into_iter() {
//        match attribute {
//            Attribute::Owner(actor_id) => {
//                owner = Some(actor_id);
//            }
//            Attribute::Permissions(perms) => {
//                permissions = Some(perms);
//            }
//            Attribute::CreatedAt(time) => {
//                created_at = Some(time);
//            }
//            Attribute::ModifiedAt(time) => {
//                modified_at = Some(time);
//            }
//            Attribute::MimeType(mime) => {
//                metadata.insert(MIME_TYPE_KEY.to_string(), mime);
//            }
//            Attribute::Custom { key, value } => {
//                metadata.insert(key, value);
//            }
//        }
//    }

//    // Validate that we have all the required attributes
//    let owner = owner.ok_or(nom::Err::Failure(NomError::new(
//        remaining,
//        ErrorKind::Verify,
//    )))?;

//    let permissions = permissions.ok_or(nom::Err::Failure(NomError::new(
//        remaining,
//        ErrorKind::Verify,
//    )))?;

//    let created_at = created_at.ok_or(nom::Err::Failure(NomError::new(
//        remaining,
//        ErrorKind::Verify,
//    )))?;

//    let modified_at = modified_at.ok_or(nom::Err::Failure(NomError::new(
//        remaining,
//        ErrorKind::Verify,
//    )))?;

//    let (remaining, content) = FileContent::parse(remaining)?;

//    let file = Self {
//        id,
//        owner,
//        permissions,
//        created_at,
//        modified_at,
//        metadata,
//        content,
//    };

//    Ok((remaining, file))
//}
//}

//#[async_trait]
//impl AsyncEncodable for File {
//    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
//        let mut written_bytes = 0;
//
//        writer.write_all(&self.id).await?;
//        written_bytes += self.id.len();
//
//        let attribute_count = 4 + self.metadata.len();
//        if attribute_count > 255 {
//            return Err(std::io::Error::new(
//                std::io::ErrorKind::InvalidData,
//                "metadata has too many entries to encode in the file",
//            ));
//        }
//
//        writer.write_all(&[attribute_count as u8]).await?;
//        written_bytes += 1;
//
//        // We know we need to order everything based on the byte, but since these have reserved
//        // types we know they'll sort before any of the other attribtues. We can take a shortcut
//        // and just encode themn directly in the order we know they'll appear.
//        written_bytes += Attribute::Owner(self.owner()).encode(writer).await?;
//        written_bytes += Attribute::Permissions(self.permissions())
//            .encode(writer)
//            .await?;
//        written_bytes += Attribute::CreatedAt(self.created_at())
//            .encode(writer)
//            .await?;
//        written_bytes += Attribute::ModifiedAt(self.modified_at())
//            .encode(writer)
//            .await?;
//
//        let mut attributes = Vec::new();
//        for (key, value) in self.metadata.iter() {
//            match key.as_str() {
//                // This is an optional named attribute that lives in the metadata and can be
//                // encoded a little more efficiently
//                "mime_type" => attributes.push(Attribute::MimeType(value.clone())),
//                _ => {
//                    attributes.push(Attribute::Custom {
//                        key: key.clone(),
//                        value: value.clone(),
//                    });
//                }
//            }
//        }
//
//        // Encode our attributes into their final byte strings. We wait here to add them into the
//        // overall encoding as they may be in any order at this point
//        let mut attribute_bytes = Vec::new();
//        for attribute in attributes.into_iter() {
//            let mut encoded_attributes = Vec::new();
//            attribute.encode(&mut encoded_attributes).await?;
//            attribute_bytes.push(encoded_attributes);
//        }
//
//        // Sort lexigraphically by the bytes strings as the RFC specifies
//        attribute_bytes.sort_unstable();
//        for attribute in attribute_bytes.into_iter() {
//            writer.write_all(&attribute).await?;
//            written_bytes += attribute.len();
//        }
//
//        written_bytes += self.content.encode(writer).await?;
//
//        Ok(written_bytes)
//    }
//}

//#[derive(Debug, thiserror::Error)]
//pub enum FileError {
//    #[error("failed to generate cid content: {0}")]
//    CidEncodingError(#[from] std::io::Error),
//}
