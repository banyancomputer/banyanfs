use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u8;

use crate::codec::{AsyncEncodable, ParserResult};

const FILE_PERMISSIONS_RESERVED_MASK: u8 = 0b1111_1000;

const FILE_PERMISSIONS_EXECUTABLE: u8 = 0b0000_0100;

const FILE_PERMISSIONS_IMMUTABLE: u8 = 0b0000_0010;

const FILE_PERMISSIONS_OWNER_WRITE_ONLY: u8 = 0b0000_0001;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions {
    executable: bool,
    immutable: bool,
    owner_write_only: bool,
}

impl FilePermissions {
    pub fn owner_write_only(&self) -> bool {
        self.owner_write_only
    }

    pub fn executable(&self) -> bool {
        self.executable
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & FILE_PERMISSIONS_RESERVED_MASK != 0 {
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        let owner_write_only = byte & FILE_PERMISSIONS_OWNER_WRITE_ONLY != 0;
        let executable = byte & FILE_PERMISSIONS_EXECUTABLE != 0;
        let immutable = byte & FILE_PERMISSIONS_IMMUTABLE != 0;

        let permissions = Self {
            owner_write_only,
            executable,
            immutable,
        };

        Ok((input, permissions))
    }
}

#[async_trait]
impl AsyncEncodable for FilePermissions {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut options: u8 = 0x00;

        if self.owner_write_only {
            options |= FILE_PERMISSIONS_OWNER_WRITE_ONLY;
        }

        if self.executable {
            options |= FILE_PERMISSIONS_EXECUTABLE;
        }

        if self.immutable {
            options |= FILE_PERMISSIONS_IMMUTABLE;
        }

        writer.write_all(&[options]).await?;

        Ok(1)
    }
}
