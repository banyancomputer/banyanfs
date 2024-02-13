mod parser_state_machine;
mod progress_type;
mod segment_streamer;
mod state_error;

pub use parser_state_machine::ParserStateMachine;
pub use progress_type::ProgressType;
pub(crate) use segment_streamer::SegmentStreamer;
pub use state_error::StateError;

pub type StateResult<T, E> = Result<ProgressType<T>, E>;
