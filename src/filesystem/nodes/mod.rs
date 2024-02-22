mod node_builder;
mod node_data;
mod node_name;

pub(crate) use node_builder::{NodeBuilder, NodeBuilderError};

pub use node_data::NodeData;
pub use node_name::{NodeName, NodeNameError};

use std::collections::HashMap;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};

use crate::codec::crypto::AccessKey;
use crate::codec::filesystem::NodeKind;
use crate::codec::meta::{ActorId, Cid, PermanentId};
use crate::codec::ParserResult;

pub(crate) type NodeId = usize;

pub struct Node {
    id: NodeId,
    parent_id: Option<NodeId>,
    cid: Option<Cid>,

    permanent_id: PermanentId,
    owner_id: ActorId,

    created_at: u64,
    modified_at: u64,

    name: NodeName,
    metadata: HashMap<String, Vec<u8>>,

    inner: NodeData,
}

impl Node {
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        data_key: Option<&AccessKey>,
    ) -> std::io::Result<(usize, Option<Vec<PermanentId>>, Option<Vec<Cid>>)> {
        let mut node_data = Vec::new();

        self.permanent_id.encode(&mut node_data).await?;
        self.owner_id.encode(&mut node_data).await?;

        let created_at_bytes = self.created_at.to_le_bytes();
        node_data.write_all(&created_at_bytes).await?;

        let modified_at_bytes = self.modified_at.to_le_bytes();
        node_data.write_all(&modified_at_bytes).await?;

        self.name.encode(&mut node_data).await?;

        let metadata_entries = self.metadata.len();
        if metadata_entries > u8::MAX as usize {
            return Err(StdError::new(
                StdErrorKind::Other,
                "too many metadata entries",
            ));
        }

        let entry_count = metadata_entries as u8;
        node_data.write_all(&[entry_count]).await?;

        let mut sorted_metadata = self.metadata.iter().collect::<Vec<_>>();
        sorted_metadata.sort_by(|(a, _), (b, _)| a.as_bytes().cmp(b.as_bytes()));

        for (key, val) in sorted_metadata.into_iter() {
            let key_bytes = key.as_bytes();
            let key_bytes_len = key_bytes.len();

            if key_bytes_len > u8::MAX as usize {
                return Err(StdError::new(StdErrorKind::Other, "metadata key too long"));
            }

            node_data.write_all(&[key_bytes_len as u8]).await?;
            node_data.write_all(key_bytes).await?;

            let val_bytes_len = val.len();
            if val_bytes_len > u8::MAX as usize {
                return Err(StdError::new(StdErrorKind::Other, "metadata val too long"));
            }

            node_data.write_all(&[val_bytes_len as u8]).await?;
            node_data.write_all(val).await?;
        }

        let (_, child_ids, data_cids) = self.data().encode(rng, &mut node_data, data_key).await?;

        let hash: [u8; 32] = blake3::hash(&node_data).into();
        let cid = Cid::from(hash);

        let mut written_bytes = 0;

        let node_data_len = (Cid::size() + node_data.len()) as u32;
        let node_data_len_bytes = node_data_len.to_le_bytes();

        writer.write_all(&node_data_len_bytes).await?;
        written_bytes += node_data_len_bytes.len();

        written_bytes += cid.encode(writer).await?;

        writer.write_all(&node_data).await?;
        written_bytes += node_data.len();

        Ok((written_bytes, child_ids, data_cids))
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn is_directory(&self) -> bool {
        self.inner.kind() == NodeKind::Directory
    }

    pub(crate) fn kind(&self) -> NodeKind {
        self.inner.kind()
    }

    pub fn data(&self) -> &NodeData {
        &self.inner
    }

    pub fn data_mut(&mut self) -> &mut NodeData {
        &mut self.inner
    }

    pub fn modified_at(&self) -> u64 {
        self.modified_at
    }

    pub fn name(&self) -> NodeName {
        self.name.clone()
    }

    pub fn owner_id(&self) -> ActorId {
        self.owner_id
    }

    pub fn parent_id(&self) -> Option<NodeId> {
        self.parent_id
    }

    pub(crate) fn parse<'a>(
        input: &'a [u8],
        data_key: Option<&AccessKey>,
    ) -> ParserResult<'a, (Self, Vec<PermanentId>)> {
        todo!()
    }

    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }

    pub fn set_attribute(&mut self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        self.metadata.insert(key, value)
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            NodeData::File { .. } => f
                .debug_tuple("NodeFile")
                .field(&self.id)
                .field(&self.cid)
                .field(&self.permanent_id)
                .field(&self.owner_id)
                .field(&self.name)
                .finish(),
            NodeData::Directory { .. } => f
                .debug_tuple("NodeDirectory")
                .field(&self.id)
                .field(&self.cid)
                .field(&self.permanent_id)
                .field(&self.owner_id)
                .field(&self.name)
                .finish(),
        }
    }
}
