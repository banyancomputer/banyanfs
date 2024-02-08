use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;

use crate::codec::header::BANYAN_DATA_MAGIC;

const ECC_PRESENT_BIT: u8 = 0b0100_0000;

const TRUNCATION_BIT: u8 = 0b1000_0000;

const BLOCK_SIZE_MASK: u8 = 0b0000_0011;

pub struct DataHeader {
    version: u8,
    data_options: DataOptions,
}

impl DataHeader {
    pub fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, (version, data_options)) = tuple((le_u8, DataOptions::parse))(input)?;

        let data_header = DataHeader {
            version,
            data_options,
        };

        Ok((input, data_header))
    }

    pub fn parse_with_magic(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, (_magic, data_header)) =
            tuple((banyan_data_magic_tag, DataHeader::parse))(input)?;
        Ok((input, data_header))
    }
}

pub(crate) struct DataOptions {
    truncated: bool,
    ecc_present: bool,
    block_size: BlockSize,
}

impl DataOptions {
    pub(crate) fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, version_byte) = take(1u8)(input)?;
        let option_byte = version_byte[0];

        let truncated = (option_byte & TRUNCATION_BIT) == TRUNCATION_BIT;
        let ecc_present = (option_byte & ECC_PRESENT_BIT) == ECC_PRESENT_BIT;

        let block_size = match option_byte & BLOCK_SIZE_MASK {
            0b00 => BlockSize::Small,
            0b01 => BlockSize::Normal,
            0b10 => BlockSize::Large,
            0b11 => BlockSize::Bulk,
            _ => unreachable!(),
        };

        let data_options = DataOptions {
            truncated,
            ecc_present,
            block_size,
        };

        Ok((input, data_options))
    }
}

pub(crate) enum BlockSize {
    /// Encoded value 0b00, this block will contain 256KiB of data.
    Small,

    /// Encoded value 0b01, this block will contain 8MiB of data.
    Normal,

    /// Encoded value 0b10, this block will contain 64MiB of data.
    Large,

    /// Encoded value 0b11, this block will contain 512MiB of data.
    Bulk,
}

fn banyan_data_magic_tag(input: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    tag(BANYAN_DATA_MAGIC)(input)
}
