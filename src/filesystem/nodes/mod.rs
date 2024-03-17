mod cid_cache;
mod node_builder;
mod node_data;
mod node_name;

pub(crate) use cid_cache::CidCache;
pub(crate) use node_builder::{NodeBuilder, NodeBuilderError};

pub use node_data::{NodeData, NodeDataError};
pub use node_name::{NodeName, NodeNameError};

use std::collections::HashMap;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::take;
use nom::number::streaming::{le_i64, le_u32, le_u8};

use crate::codec::crypto::AccessKey;
use crate::codec::filesystem::NodeKind;
use crate::codec::meta::{ActorId, Cid, PermanentId};
use crate::codec::ParserResult;
use crate::filesystem::drive::OperationError;

pub(crate) type NodeId = usize;

pub struct Node {
    id: NodeId,

    cid: CidCache,
    parent_id: Option<PermanentId>,

    permanent_id: PermanentId,
    owner_id: ActorId,

    created_at: i64,
    modified_at: i64,

    name: NodeName,
    metadata: HashMap<String, Vec<u8>>,

    inner: NodeData,
}

impl Node {
    pub async fn add_child(
        &mut self,
        name: NodeName,
        child_id: PermanentId,
    ) -> Result<(), NodeError> {
        self.inner.add_child(name, child_id)?;

        self.cid.mark_dirty().await;
        self.modified_at = crate::utils::current_time_ms();

        Ok(())
    }

    pub async fn cached_encoding(&self) -> Option<Vec<u8>> {
        self.cid.take_cached().await
    }

    pub(crate) fn child_pids(&self) -> Option<Vec<PermanentId>> {
        self.inner.child_pids()
    }

    pub async fn cid(&self) -> Result<Cid, OperationError> {
        // todo(sstelfox): this should always return a CID, if we can't get it
        // from the cache we should encode ourselves, cache it and return the
        // generated CID.
        todo!()
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn data(&self) -> &NodeData {
        &self.inner
    }

    pub async fn data_mut(&mut self) -> &mut NodeData {
        // note(sstelfox): we should probably arbitrate changes to the inner data in a way that
        // allows us to detect if changes actually occurred. For now we have to assume that by
        // grabbing a mutable handle they intend to mutate the content which will invalidate our
        // CID so we mark ourselves as dirty.
        self.cid.mark_dirty().await;
        &mut self.inner
    }

    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
        data_key: Option<&AccessKey>,
    ) -> std::io::Result<(usize, Vec<PermanentId>, Vec<Cid>)> {
        let mut node_data = Vec::new();

        match self.parent_id {
            Some(pid) => {
                node_data.write_all(&[0x01]).await?;
                pid.encode(&mut node_data).await?;
            }
            None => {
                node_data.write_all(&[0x00]).await?;
            }
        };

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

        // todo(sstelfox): not sure if this is quite the right place, but I need to make sure that
        // child CIDs are referenced alongside their permanent IDs in the child hierarchy so we can
        // detect whole tree changes.
        let (data_len, ordered_child_ids, ordered_data_cids) =
            self.data().encode(rng, &mut node_data, data_key).await?;
        tracing::trace!(node_data_len = data_len, "node_data::encoded");

        self.cid.set_with_ref(&node_data).await;

        let mut written_bytes = 0;

        let cid = self
            .cid
            .cid()
            .await
            .map_err(|_| StdError::new(StdErrorKind::Other, "failed to get CID"))?;
        written_bytes += cid.encode(writer).await?;

        let node_data_len = node_data.len() as u32;
        let node_data_len_bytes = node_data_len.to_le_bytes();

        writer.write_all(&node_data_len_bytes).await?;
        written_bytes += node_data_len_bytes.len();

        writer.write_all(&node_data).await?;
        written_bytes += node_data.len();

        Ok((written_bytes, ordered_child_ids, ordered_data_cids))
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub(crate) fn kind(&self) -> NodeKind {
        self.inner.kind()
    }

    pub fn metadata(&self) -> &HashMap<String, Vec<u8>> {
        &self.metadata
    }

    pub fn modified_at(&self) -> i64 {
        self.modified_at
    }

    pub fn name(&self) -> NodeName {
        self.name.clone()
    }

    pub fn owner_id(&self) -> ActorId {
        self.owner_id
    }

    pub fn parent_id(&self) -> Option<PermanentId> {
        self.parent_id
    }

    #[tracing::instrument(skip(input))]
    pub(crate) fn parse<'a>(
        input: &'a [u8],
        allocated_id: NodeId,
        data_key: Option<&AccessKey>,
    ) -> ParserResult<'a, (Self, Vec<PermanentId>)> {
        tracing::trace!(allocated_id, "begin");

        let (input, cid) = Cid::parse(input)?;
        let (input, node_data_len) = le_u32(input)?;
        tracing::trace!(node_data_len, ?cid, "cid/node_data_len");

        let (input, node_data_buf) = take(node_data_len)(input)?;

        let (node_data_buf, parent_present) = take(1u8)(node_data_buf)?;
        let (node_data_buf, parent_id) = match parent_present[0] {
            0x00 => (node_data_buf, None),
            0x01 => {
                let (node_data_buf, pid) = PermanentId::parse(node_data_buf)?;
                (node_data_buf, Some(pid))
            }
            _ => {
                let err = nom::error::make_error(input, nom::error::ErrorKind::Switch);
                return Err(nom::Err::Failure(err));
            }
        };

        let (node_data_buf, permanent_id) = PermanentId::parse(node_data_buf)?;
        let (node_data_buf, owner_id) = ActorId::parse(node_data_buf)?;
        let (node_data_buf, created_at) = le_i64(node_data_buf)?;
        let (node_data_buf, modified_at) = le_i64(node_data_buf)?;
        let (node_data_buf, name) = NodeName::parse(node_data_buf)?;
        let (mut node_data_buf, metadata_entries) = le_u8(node_data_buf)?;

        let mut metadata = HashMap::new();
        for _ in 0..metadata_entries {
            let (meta_buf, key_len) = le_u8(node_data_buf)?;
            let (meta_buf, key) = take(key_len)(meta_buf)?;
            let key_str = String::from_utf8(key.to_vec()).map_err(|_| {
                nom::Err::Failure(nom::error::make_error(input, nom::error::ErrorKind::Char))
            })?;

            let (meta_buf, val_len) = le_u8(meta_buf)?;
            let (meta_buf, val) = take(val_len)(meta_buf)?;
            let val = val.to_vec();

            metadata.insert(key_str, val);
            node_data_buf = meta_buf;
        }

        let (remaining, (inner, desired_node_ids)) = NodeData::parse(node_data_buf, data_key)?;
        debug_assert!(remaining.is_empty());

        let node = Self {
            id: allocated_id,

            cid: CidCache::empty(),
            parent_id,

            permanent_id,
            owner_id,

            created_at,
            modified_at,

            name,
            metadata,

            inner,
        };

        Ok((input, (node, desired_node_ids)))
    }

    pub(crate) async fn remove_child(&mut self, child_name: &NodeName) -> Result<(), NodeError> {
        self.inner.remove_child(child_name)?;

        self.cid.mark_dirty().await;
        self.modified_at = crate::utils::current_time_ms();

        Ok(())
    }

    pub(crate) async fn remove_permanent_id(
        &mut self,
        child_id: &PermanentId,
    ) -> Result<(), NodeError> {
        self.inner.remove_permanent_id(child_id)?;

        self.cid.mark_dirty().await;
        self.modified_at = crate::utils::current_time_ms();

        Ok(())
    }

    pub(crate) async fn set_name(&mut self, new_name: NodeName) {
        self.name = new_name;

        self.cid.mark_dirty().await;
        self.modified_at = crate::utils::current_time_ms();
    }

    pub(crate) async fn set_parent_id(&mut self, parent_id: PermanentId) {
        self.parent_id = Some(parent_id);

        self.cid.mark_dirty().await;
        self.modified_at = crate::utils::current_time_ms();
    }

    pub(crate) fn supports_children(&self) -> bool {
        match self.inner.kind() {
            NodeKind::Directory => true,
            NodeKind::File => true,
            _ => false,
        }
    }

    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }

    pub async fn set_attribute(&mut self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        let old_value = self.metadata.insert(key, value);

        self.cid.mark_dirty().await;
        self.modified_at = crate::utils::current_time_ms();

        old_value
    }
}

// TODO: this belongs on the inner class not here
impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            NodeData::File { .. } => f
                .debug_tuple("NodeFile")
                .field(&self.id)
                .field(&self.permanent_id)
                .field(&self.owner_id)
                .field(&self.name)
                .finish(),
            NodeData::AssoicatedData => unimplemented!(),
            NodeData::Directory { .. } => f
                .debug_tuple("NodeDirectory")
                .field(&self.id)
                .field(&self.permanent_id)
                .field(&self.owner_id)
                .field(&self.name)
                .finish(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("failed manipulating inner node data: {0}")]
    NodeDataError(#[from] NodeDataError),

    #[error("attempted to perform a child operation on a node without children")]
    HasNoChildren,
}
