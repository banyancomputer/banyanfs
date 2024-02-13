use elliptic_curve::rand_core::CryptoRngCore;
use slab::Slab;

use crate::codec::crypto::SigningKey;
use crate::codec::header::KeyAccessSettingsBuilder;
use crate::codec::meta::{ActorId, FilesystemId};
use crate::filesystem::{DriveAccess, Entry, EntryBuilder, EntryId};

pub struct Drive {
    filesystem_id: FilesystemId,
    access: DriveAccess,

    nodes: Slab<Entry>,
    root_entry_id: EntryId,
}

impl Drive {
    pub fn has_realized_view_access(&self, actor_id: ActorId) -> bool {
        self.access.has_realized_view_access(actor_id)
    }

    pub fn has_write_access(&self, actor_id: ActorId) -> bool {
        self.access.has_write_access(actor_id)
    }

    //pub async fn encode_private<W: AsyncWrite + Unpin + Send>(
    //    &self,
    //    rng: &mut impl CryptoRngCore,
    //    writer: &mut W,
    //    _signing_key: &SigningKey,
    //) -> std::io::Result<usize> {
    //    let mut written_bytes = 0;

    //    written_bytes += IdentityHeader::encode(&IdentityHeader, writer).await?;
    //    written_bytes += self.filesystem_id.encode(writer).await?;

    //    // Don't support ECC yet
    //    written_bytes += PublicSettings::new(false, true).encode(writer).await?;

    //    let encoding_context = PrivateEncodingContext::new(
    //        rng,
    //        self.keys.clone(),
    //        (0, 0),
    //        (Cid::from([0u8; 32]), Cid::from([0u8; 32])),
    //    );

    //    let content_payload = ContentPayload::Private;
    //    written_bytes += content_payload
    //        .encode_private(rng, &encoding_context, writer)
    //        .await?;

    //    Ok(written_bytes)
    //}

    pub fn id(&self) -> FilesystemId {
        self.filesystem_id
    }

    pub fn initialize_private(rng: &mut impl CryptoRngCore, signing_key: &SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        let actor_id = verifying_key.actor_id();

        let kas = KeyAccessSettingsBuilder::private()
            .set_owner()
            .set_protected()
            .build();

        let mut access = DriveAccess::default();
        access.register_actor(verifying_key, kas);

        let mut nodes = Slab::with_capacity(32);

        let root_entry = nodes.vacant_entry();
        let root_entry_id = root_entry.key();

        let directory = EntryBuilder::directory(root_entry_id, actor_id).build(rng);
        root_entry.insert(directory);

        Self {
            filesystem_id: FilesystemId::generate(rng),
            access,

            nodes,
            root_entry_id,
        }
    }
}

//impl Deref for Drive {
//    type Target = Directory;
//
//    fn deref(&self) -> &Self::Target {
//        &self.root
//    }
//}
//
//impl DerefMut for Drive {
//    fn deref_mut(&mut self) -> &mut Self::Target {
//        &mut self.root
//    }
//}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error("failed to parse drive data, is this a banyanfs file?")]
    HeaderReadFailure,
}
