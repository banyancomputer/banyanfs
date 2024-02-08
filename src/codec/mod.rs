mod cid;
mod content_payload;
pub(crate) mod crypto;
pub mod header;

use std::future::Future;
use std::pin::Pin;

use tokio::io::AsyncWrite;

#[async_trait::async_trait]
pub trait AsyncEncodable {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> std::io::Result<usize>;
}
