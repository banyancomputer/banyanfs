mod actor_id;
mod cid;
pub mod content_payload;
pub mod crypto;
pub mod filesystem;
mod filesystem_id;
pub mod header;
mod segment_streamer;

use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite};

pub use actor_id::ActorId;
pub use cid::Cid;
pub use filesystem_id::FilesystemId;
pub use segment_streamer::*;

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

pub type ParserResult<'a, T> = nom::IResult<&'a [u8], T>;

#[async_trait]
pub trait AsyncParse<'a>: Parser + Sized {
    async fn next(mut input: &'a [u8], ctx: &'a Self::Context) -> ParserResult<'a, Option<Self>> {
        match Self::parse(&input, ctx) {
            Ok((remaining, parsed)) => Ok((remaining, Some(parsed))),
            Err(nom::Err::Incomplete(_)) => Ok((input, None)),
            Err(err) => Err(err),
        }
    }
}

#[async_trait]
pub trait AsyncEncodable {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize>;
}
