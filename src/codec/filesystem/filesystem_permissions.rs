use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::le_u8;
use nom::IResult;

use crate::codec::AsyncEncodable;

const PERMISSIONS_RESERVED_MASK: u8 = 0b1111_1000;

const PERMISSIONS_EXECUTABLE: u8 = 0b0000_0100;

const PERMISSIONS_IMMUTABLE: u8 = 0b0000_0010;

const PERMISSIONS_CREATOR_WRITE_ONLY: u8 = 0b0000_0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilesystemPermissions {
    creator_write_only: bool,
    executable: bool,
    immutable: bool,
}

impl FilesystemPermissions {
    pub fn creator_write_only(&self) -> bool {
        self.creator_write_only
    }

    pub fn executable(&self) -> bool {
        self.executable
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }
}

impl FilesystemPermissions {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & PERMISSIONS_RESERVED_MASK != 0 {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Tag)));
        }

        let creator_write_only = byte & PERMISSIONS_CREATOR_WRITE_ONLY != 0;
        let executable = byte & PERMISSIONS_EXECUTABLE != 0;
        let immutable = byte & PERMISSIONS_IMMUTABLE != 0;

        let permissions = Self {
            creator_write_only,
            executable,
            immutable,
        };

        Ok((input, permissions))
    }
}

#[async_trait]
impl AsyncEncodable for FilesystemPermissions {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        _pos: usize,
    ) -> std::io::Result<usize> {
        let mut options: u8 = 0x00;

        if self.creator_write_only {
            options |= PERMISSIONS_CREATOR_WRITE_ONLY;
        }

        if self.executable {
            options |= PERMISSIONS_EXECUTABLE;
        }

        if self.immutable {
            options |= PERMISSIONS_IMMUTABLE;
        }

        writer.write_all(&[options]).await?;

        Ok(1)
    }
}
