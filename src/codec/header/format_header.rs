use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;

use crate::codec::header::{FilesystemId, IdentityHeader, PublicSettings};

pub(crate) struct FormatHeader {
    ecc_present: bool,
    private: bool,
    filesystem_id: FilesystemId,
}

impl FormatHeader {
    pub(crate) fn parse_with_magic(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let mut header_parser = tuple((
            IdentityHeader::parse_with_magic,
            FilesystemId::parse,
            PublicSettings::parse,
        ));

        let (input, (_, filesystem_id, settings)) = header_parser(input)?;

        let header = FormatHeader {
            ecc_present: settings.ecc_present(),
            private: settings.private(),
            filesystem_id,
        };

        Ok((input, header))
    }
}
