use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::sequence::tuple;

mod header;

pub(crate) use header::{DataHeader, IdentityHeader};

pub(crate) struct Cid([u8; 32]);

impl Cid {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(crate) fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        todo!()
    }
}
