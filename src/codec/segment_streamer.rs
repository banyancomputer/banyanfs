use bytes::{Bytes, BytesMut};
use futures::stream::Stream;
use futures::FutureExt;

pub trait StateError {
    fn needed_data(&self) -> Option<usize>;

    fn needs_more_data(&self) -> bool;
}

pub trait ParserStateMachine<T> {
    type Error: StateError;

    fn parse(&mut self, buffer: &[u8]) -> StateResult<T, Self::Error>;
}

pub type StateResult<T, E> = Result<ProgressType<T>, E>;

pub enum ProgressType<T> {
    Ready(usize, T),
    Advance(usize),
}

pub struct SegmentStreamer<T, S: ParserStateMachine<T>> {
    buffer: BytesMut,
    state_machine: S,
    hasher: blake3::Hasher,

    _phantom: std::marker::PhantomData<T>,
}

impl<T: Unpin, S: ParserStateMachine<T>> SegmentStreamer<T, S> {
    pub fn add_chunk(&mut self, chunk: &Bytes) {
        self.buffer.extend_from_slice(chunk);
    }

    pub fn new(initial_state: S) -> Self {
        Self {
            buffer: bytes::BytesMut::new(),
            state_machine: initial_state,
            hasher: blake3::Hasher::new(),

            _phantom: std::marker::PhantomData,
        }
    }

    pub fn reset_digest(&mut self) {
        self.hasher.reset();
    }

    pub async fn next(&mut self) -> Option<Result<([u8; 32], T), S::Error>> {
        loop {
            match self.state_machine.parse(&self.buffer) {
                Ok(ProgressType::Ready(byte_count, val)) => {
                    let read_data = self.buffer.split_to(byte_count);

                    self.hasher.update(&read_data);
                    let hash = self.hasher.finalize();
                    let byte_hash: [u8; 32] = hash.into();
                    self.reset_digest();

                    return Some(Ok((byte_hash, val)));
                }
                Ok(ProgressType::Advance(byte_count)) => {
                    let read_data = self.buffer.split_to(byte_count);
                    self.hasher.update(&read_data);
                }
                Err(err) if err.needs_more_data() => return None,
                Err(err) => return Some(Err(err)),
            }
        }
    }
}

impl<T: Unpin, S: ParserStateMachine<T> + Unpin> Stream for SegmentStreamer<T, S> {
    type Item = Result<([u8; 32], T), S::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let self_mut = self.get_mut();

        let fut = self_mut.next();
        let mut fut = Box::pin(fut);
        fut.poll_unpin(cx)
    }
}
