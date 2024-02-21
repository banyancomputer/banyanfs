use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u8;

use crate::codec::ParserResult;

const DIRECTORY_PERMISSIONS_RESERVED_MASK: u8 = 0b1111_1100;

const DIRECTORY_PERMISSIONS_IMMUTABLE: u8 = 0b0000_0010;

const DIRECTORY_PERMISSIONS_OWNER_WRITE_ONLY: u8 = 0b0000_0001;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DirectoryPermissions {
    immutable: bool,
    owner_write_only: bool,
}

impl DirectoryPermissions {
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut options: u8 = 0x00;

        if self.owner_write_only {
            options |= DIRECTORY_PERMISSIONS_OWNER_WRITE_ONLY;
        }

        if self.immutable {
            options |= DIRECTORY_PERMISSIONS_IMMUTABLE;
        }

        writer.write_all(&[options]).await?;

        Ok(1)
    }
    pub fn owner_write_only(&self) -> bool {
        self.owner_write_only
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & DIRECTORY_PERMISSIONS_RESERVED_MASK != 0 {
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        let owner_write_only = byte & DIRECTORY_PERMISSIONS_OWNER_WRITE_ONLY != 0;
        let immutable = byte & DIRECTORY_PERMISSIONS_IMMUTABLE != 0;

        let permissions = Self {
            owner_write_only,
            immutable,
        };

        Ok((input, permissions))
    }
}
