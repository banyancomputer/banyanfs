mod actor_id;
mod cid;
pub mod content_payload;
pub mod crypto;
pub mod filesystem;
mod filesystem_id;
pub mod header;

use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite};

pub use actor_id::ActorId;
pub use cid::Cid;
pub use filesystem_id::FilesystemId;

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

//use bytes::BufMut;
//
//#[async_trait]
//pub trait AsyncParse<'a>: Parser + Sized {
//    async fn async_parse<R: AsyncRead + AsyncReadExt + Unpin + Send>(
//        reader: &mut R,
//        ctx: &'a Self::Context,
//    ) -> ParserResult<Self> {
//        let mut buffer = bytes::BytesMut::with_capacity(1024); // Adjust initial capacity as needed
//        let mut needed = 0;
//
//        loop {
//            if buffer.len() < needed {
//                let bytes_read = match reader.read(&mut buffer).await {
//                    Ok(bytes_read) => bytes_read,
//                    Err(err) => {
//                        tracing::error!("encountered an i/o error: {err}");
//                        return Err(nom::Err::Error(nom::error::Error::new(
//                            buffer.as_ref(),
//                            nom::error::ErrorKind::Eof,
//                        )));
//                    }
//                };
//
//                if bytes_read == 0 {
//                    return Err(nom::Err::Error(nom::error::Error::new(
//                        buffer.as_ref(),
//                        nom::error::ErrorKind::Eof,
//                    )));
//                }
//            }
//
//            match Self::parse(&buffer, ctx) {
//                Ok(parsed) => return Ok(parsed),
//                Err(nom::Err::Incomplete(nom::Needed::Size(n))) => needed = buffer.len() + n.get(),
//                Err(nom::Err::Incomplete(_)) => needed += 1,
//                Err(err) => return Err(err),
//            }
//        }
//    }
//}

#[async_trait]
pub trait AsyncEncodable {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize>;
}
