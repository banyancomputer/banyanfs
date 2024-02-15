pub mod content_payload;
pub mod crypto;
pub mod filesystem;
pub mod header;
pub mod meta;
pub mod parser;

use async_trait::async_trait;
use futures::AsyncWrite;

pub use meta::*;
pub use parser::*;

#[async_trait]
pub trait AsyncEncodable {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize>;
}
