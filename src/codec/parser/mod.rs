mod parser_state_machine;
mod progress_type;
mod segment_streamer;
mod state_error;

pub use parser_state_machine::ParserStateMachine;
pub use progress_type::ProgressType;
pub(crate) use segment_streamer::SegmentStreamer;
pub use state_error::StateError;

use async_trait::async_trait;

pub type ParserResult<'a, T> = nom::IResult<&'a [u8], T>;

pub type StateResult<T, E> = Result<ProgressType<T>, E>;

#[async_trait]
pub trait AsyncParser<'a>: Parser + Sized {
    async fn next(input: &'a [u8], ctx: &'a Self::Context) -> ParserResult<'a, Option<Self>> {
        match Self::parse(input, ctx) {
            Ok((remaining, parsed)) => Ok((remaining, Some(parsed))),
            Err(nom::Err::Incomplete(_)) => Ok((input, None)),
            Err(err) => Err(err),
        }
    }
}

pub trait Parser: Sized {
    type Context: Send + Sync;

    fn parse<'a>(input: &'a [u8], ctx: &'a Self::Context) -> ParserResult<'a, Self>;

    fn parse_many<'a>(
        mut input: &'a [u8],
        ctx: &'a Self::Context,
        count: usize,
    ) -> ParserResult<'a, Vec<Self>> {
        let mut collection = Vec::with_capacity(count);

        for _ in 0..count {
            let (remaining, item) = Self::parse(input, ctx)?;
            collection.push(item);
            input = remaining;
        }

        Ok((input, collection))
    }
}
