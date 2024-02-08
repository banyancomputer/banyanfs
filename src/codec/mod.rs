pub mod cid;
pub mod content_payload;
pub mod crypto;
pub mod header;

use async_trait::async_trait;
use futures::AsyncWrite;

#[async_trait]
pub trait AsyncEncodable {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> std::io::Result<usize>;
}
