use crate::codec::parser::{StateError, StateResult};

use super::Stream;

pub trait ParserStateMachine<T> {
    type Error: StateError;

    fn parse(&mut self, buffer: Stream) -> StateResult<T, Self::Error>;
}
