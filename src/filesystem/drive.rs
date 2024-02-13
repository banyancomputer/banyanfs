use slab::Slab;
use std::collections::HashMap;

use crate::codec::crypto::VerifyingKey;
use crate::codec::FilesystemId;
use crate::filesystem::DriveAccess;
use crate::filesystem::FilesystemEntry;

pub struct Drive {
    filesystem_id: FilesystemId,
    access: DriveAccess,

    nodes: Slab<FilesystemEntry>,
    id_map: HashMap<[u8; 16], EntryId>,

    root: EntryId,
}

type EntryId = usize;

impl Drive {
    pub fn has_write_access(&self, key: &VerifyingKey) -> bool {
        self.access.has_write_access(key.actor_id())
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

    //pub fn initialize_private(rng: &mut impl CryptoRngCore, signing_key: &SigningKey) -> Self {
    //    let verifying_key = signing_key.verifying_key();
    //    let actor_id = signing_key.actor_id();

    //    let kas = KeyAccessSettings::Private {
    //        protected: true,
    //        owner: true,
    //        historical: false,

    //        realized_key_present: true,
    //        data_key_present: true,
    //        journal_key_present: true,
    //        maintenance_key_present: true,
    //    };

    //    let mut keys = HashMap::new();
    //    keys.insert(actor_id, (verifying_key, kas));

    //    Self {
    //        filesystem_id: FilesystemId::generate(rng),
    //        keys,
    //        root: Directory::new(rng, actor_id),
    //    }
    //}
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
