use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;

const ECC_PRESENT_BIT: u8 = 0x02;

const PRIVATE_BIT: u8 = 0x01;

const RESERVED_BITS: u8 = 0xfc;

pub(crate) struct PublicSettings {
    ecc_present: bool,
    private: bool,
}

impl PublicSettings {
    pub(crate) fn ecc_present(&self) -> bool {
        self.ecc_present
    }

    pub(crate) fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, settings_byte) = take(1u8)(input)?;
        let settings_byte = settings_byte[0];

        if cfg!(feature = "strict") && (settings_byte & RESERVED_BITS) != 0 {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)));
        }

        let ecc_present = (settings_byte & ECC_PRESENT_BIT) == ECC_PRESENT_BIT;
        let private = (settings_byte & PRIVATE_BIT) == PRIVATE_BIT;

        let settings = Self {
            ecc_present,
            private,
        };

        Ok((input, settings))
    }

    pub(crate) fn private(&self) -> bool {
        self.private
    }
}