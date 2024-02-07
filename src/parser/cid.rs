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

        // All zeros and all ones are disallowed, this isn't actually harmful though so we'll only
        // perform this check in strict mode.
        if cfg!(feature = "strict")
            && (cid_bytes.iter().all(|&b| b == 0x00) || cid_bytes.iter().all(|&b| b == 0xff))
        {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Verify)));
        }

        let mut bytes = [0u8; CID_LENGTH];
        bytes.copy_from_slice(cid_bytes);

        Ok((remaining, Self(bytes)))
    }
}
