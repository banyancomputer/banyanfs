mod parser_state_machine;
mod progress_type;
mod segment_streamer;
mod state_error;

pub use parser_state_machine::ParserStateMachine;
pub use progress_type::ProgressType;
pub(crate) use segment_streamer::SegmentStreamer;
pub use state_error::StateError;
use winnow::Partial;

pub type Stream<'a> = Partial<&'a [u8]>;

#[cfg(debug_assertions)]
pub type ParserResult<'a, T> =
    winnow::IResult<Stream<'a>, T, winnow::error::VerboseError<Stream<'a>>>;

pub type CompleteParserResult<'a, T> =
    winnow::IResult<&'a [u8], T, winnow::error::VerboseError<&'a [u8]>>;

#[cfg(not(debug_assertions))]
pub type ParserResult<'a, T> = winnow::IResult<Stream<'a>, T>;
#[cfg(not(debug_assertions))]
pub type CompleteParserResult<'a, T> = winnow::IResult<&'a [u8], T>;

pub type StateResult<T, E> = Result<ProgressType<T>, E>;

//#[async_trait]
//pub trait AsyncParser<'a>: Parser + Sized {
//    async fn next(input: &'a [u8], ctx: &'a Self::Context) -> ParserResult<'a, Option<Self>> {
//        match Self::parse(input, ctx) {
//            Ok((remaining, parsed)) => Ok((remaining, Some(parsed))),
//            Err(winnow::error::ErrMode::Incomplete(_)) => Ok((input, None)),
//            Err(err) => Err(err),
//        }
//    }
//}

pub trait Parser: Sized {
    type Context: Send + Sync;

    fn parse<'a>(input: Stream<'a>, ctx: &'a Self::Context) -> ParserResult<'a, Self>;

    fn parse_many<'a>(
        mut input: Stream<'a>,
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
