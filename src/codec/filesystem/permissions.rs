use futures::{AsyncWrite, AsyncWriteExt};
use winnow::{binary::le_u8, Parser};

use crate::codec::{ParserResult, Stream};

const PERMISSIONS_RESERVED_MASK: u8 = 0b1111_1000;

const PERMISSIONS_EXECUTABLE: u8 = 0b0000_0100;

const PERMISSIONS_IMMUTABLE: u8 = 0b0000_0010;

const PERMISSIONS_OWNER_WRITE_ONLY: u8 = 0b0000_0001;

// todo(sstelfox): We only need one type of permission, they can be shared to simplify the
// protocol.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    executable: bool,
    immutable: bool,
    owner_write_only: bool,
}

impl Permissions {
    pub fn owner_write_only(&self) -> bool {
        self.owner_write_only
    }

    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut options: u8 = 0x00;

        if self.owner_write_only {
            options |= PERMISSIONS_OWNER_WRITE_ONLY;
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

    pub fn executable(&self) -> bool {
        self.executable
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (input, byte) = le_u8.parse_peek(input)?;

        if cfg!(feature = "strict") && byte & PERMISSIONS_RESERVED_MASK != 0 {
            let err = winnow::error::ParserError::from_error_kind(
                &input,
                winnow::error::ErrorKind::Verify,
            );
            return Err(winnow::error::ErrMode::Cut(err));
        }

        let owner_write_only = byte & PERMISSIONS_OWNER_WRITE_ONLY != 0;
        let executable = byte & PERMISSIONS_EXECUTABLE != 0;
        let immutable = byte & PERMISSIONS_IMMUTABLE != 0;

        let permissions = Self {
            owner_write_only,
            executable,
            immutable,
        };

        Ok((input, permissions))
    }

    pub const fn size() -> usize {
        1
    }
}
