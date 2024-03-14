use std::pin::Pin;
use std::task::{Context, Poll};

use async_std::stream::Stream;
use bytes::Bytes;

pub(crate) fn vec_to_pinned_stream(
    inner: Vec<u8>,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>>>> {
    Box::pin(VecStream::new(inner))
}

pub(crate) struct VecStream {
    data: Vec<u8>,
    pos: usize,
    length: usize,
}

impl VecStream {
    pub(crate) fn new(data: Vec<u8>) -> Self {
        let length = data.len();

        Self {
            data,
            pos: 0,
            length,
        }
    }
}

impl Stream for VecStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = self.get_mut();

        if inner.pos >= inner.data.len() {
            return Poll::Ready(None);
        }

        let end_pos = inner.data.len();
        let bytes = Bytes::copy_from_slice(&inner.data[inner.pos..end_pos]);
        inner.pos = end_pos;

        Poll::Ready(Some(Ok(bytes)))
    }
}
