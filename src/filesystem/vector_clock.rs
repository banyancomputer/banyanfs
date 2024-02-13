use nom::number::streaming::le_u64;
use nom::IResult;

#[derive(Debug)]
pub struct VectorClock(u64);

impl VectorClock {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = le_u64(input)?;
        Ok((input, Self(value)))
    }
}
