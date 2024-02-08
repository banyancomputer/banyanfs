use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::sequence::tuple;

const CID_LENGTH: usize = 32;

pub(crate) struct Cid([u8; CID_LENGTH]);

impl Cid {
    pub fn as_bytes(&self) -> &[u8; CID_LENGTH] {
        &self.0
    }

    pub(crate) fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (remaining, cid_bytes) = take(CID_LENGTH)(input)?;

        let mut bytes = [0u8; CID_LENGTH];
        bytes.copy_from_slice(cid_bytes);

        Ok((remaining, Self(bytes)))
    }
}
