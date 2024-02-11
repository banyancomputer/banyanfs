mod actor_id;
mod cid;
pub mod content_payload;
pub mod crypto;
pub mod filesystem;
mod filesystem_id;
pub mod header;

use async_trait::async_trait;
use futures::AsyncWrite;

pub use actor_id::ActorId;
pub use cid::Cid;
pub use filesystem_id::FilesystemId;

#[async_trait]
pub trait AsyncEncodable {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        pos: usize,
    ) -> std::io::Result<usize>;
}
