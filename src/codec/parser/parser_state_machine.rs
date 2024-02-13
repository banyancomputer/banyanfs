use crate::codec::parser::{StateError, StateResult};

pub trait ParserStateMachine<T> {
    type Error: StateError;

    fn parse(&mut self, buffer: &[u8]) -> StateResult<T, Self::Error>;
}
