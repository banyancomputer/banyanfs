pub mod content_payload;
pub mod crypto;
mod filesystem_id;
pub mod header;

use async_trait::async_trait;
use futures::AsyncWrite;

pub use filesystem_id::FilesystemId;

#[async_trait]
pub trait AsyncEncodable {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> std::io::Result<usize>;
}
