//! # Nodes
//!
//! Somewhere between on disk storage format, and objects being operated on within the filesystem,
//! a [`Node`] is an entry within the filesystem itself. Any component or type that are core to a
//! node's operations belong within this module while simple data structures should exist within
//! the [`crate::codec`] module.
//!
//! There are some unsorted data structures that still need to be organized elsewhere.
//!
//! Generally this module represents internal operations and the details shouldn't be exposed to
//! consumers of the library. The inner workings can be useful for advanced users but these APIs
//! are not considered part of our public API for the purpose of breaking changes (we won't
//! guarantee the major version will be increased when a breaking change is made).

mod encoded_cache;
mod node_builder;
mod node_context;
mod node_data;
mod node_name;

use async_std::sync::{RwLock, RwLockReadGuard};
pub(crate) use encoded_cache::EncodedCache;
pub(crate) use node_builder::{NodeBuilder, NodeBuilderError};

pub(crate) use node_context::NodeContext;
pub(crate) use node_data::{NodeData, NodeDataError};
pub(crate) use node_name::{NodeName, NodeNameError};

use futures::{AsyncWrite, AsyncWriteExt};
use parking_lot::MappedRwLockReadGuard;
use std::collections::HashMap;
use std::mem::size_of;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};
use winnow::binary::{le_i64, le_u32, le_u8};
use winnow::stream::Offset;
use winnow::token::take;
use winnow::Parser;

use crate::codec::crypto::AccessKey;
use crate::codec::filesystem::NodeKind;
use crate::codec::meta::{ActorId, Cid, PermanentId};
use crate::codec::{ParserResult, Stream, VectorClock};
use crate::filesystem::drive::OperationError;
use crate::utils::calculate_cid;

pub(crate) type NodeId = usize;

/// The core structure that represents a node within the filesystem. This structure represents a
/// wrapper over any kind of filesystem metadata, structure, or associated data contained within
/// the BanyanFS structures.
pub struct Node {
    id: NodeId,
    parent_id: Option<PermanentId>,
    permanent_id: PermanentId,
    owner_id: ActorId,

    encoded_cache: EncodedCache,
    vector_clock: VectorClock,

    created_at: i64,
    modified_at: i64,

    name: NodeName,
    metadata: HashMap<String, Vec<u8>>,

    inner: NodeData,
}

impl Node {
    /// Associates a child node with the current node. This is a low-level operation and should be
    /// used with care. This will check that the current nodes allow a child below it, but does not
    /// validate the node being added is a valid type. It is the responsibility of the caller to
    /// ensure that for example a directory is not added below a child. The correct one-way
    /// hierarchy is directory -> (directory | file), file -> associated data.
    ///
    /// Calling this function will modify the CID of the node and as such will invalidate the
    /// internal encoding cache if available from [`Node::cached_encoding`].
    pub(crate) async fn add_child(
        &mut self,
        name: NodeName,
        child_id: PermanentId,
    ) -> Result<(), NodeDataError> {
        self.inner.add_child(name, child_id)?;
        self.notify_of_change().await;

        Ok(())
    }

    /// Returns the CID of the node. If the internal data has changed in anyway (as indicated by
    /// and internal call to CidCache::is_dirty), this will fully encode the node as it would
    /// appear on disk and calculates the CID over that data.
    ///
    /// As an optimization this caches that encoding so we don't have to re-encode it when we're
    /// writing the filesystem out to disk. This comes with a small memory penalty if some
    /// non-encoding process attempts to access a large number of node CIDs but that seems like an
    /// unlikely use case.
    pub async fn cid(&self) -> Result<Cid, OperationError> {
        let encoded = self.encoded().await.map_err(|_| {
            OperationError::InternalCorruption(self.id, "failed to encode node for CID")
        })?;
        Ok(Cid::try_from(&encoded[..Cid::size()]).expect("Cid size is constant"))
    }

    /// Returns the unix timestamp (in milliseconds precision) of when the node was created.
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub(crate) fn data(&self) -> &NodeData {
        &self.inner
    }

    pub(crate) async fn data_mut(&mut self) -> &mut NodeData {
        // note(sstelfox): we should probably arbitrate changes to the inner data in a way that
        // allows us to detect if changes actually occurred. For now we have to assume that by
        // grabbing a mutable handle they intend to mutate the content which will invalidate our
        // CID so we mark ourselves as dirty.
        self.notify_of_change().await;
        &mut self.inner
    }

    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let encoded = self.encoded().await?;
        let mut written_bytes = 0;


        writer.write_all(&encoded).await?;
        written_bytes += encoded.len();

        Ok(written_bytes)
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

    pub(crate) async fn notify_of_change(&mut self) {
        self.encoded_cache.mark_dirty();
        self.modified_at = crate::utils::current_time_ms();
    }

    pub(crate) fn ordered_child_pids(&self) -> Vec<PermanentId> {
        self.inner.ordered_child_pids()
    }

    pub(crate) fn data_cids(&self) -> Option<Vec<Cid>> {
        self.inner.data_cids()
    }

    /// This returns the estimated amount of storage that is taken up by attributes at this level of
    /// indirection without the contents of the data itself. This is used internally to dynamically
    /// estimate of the total encoding size of the node.
    fn outer_size_estimate(&self) -> u64 {
        let mut encoded_size = Cid::size(); // Cid
        encoded_size += 4; // Node data Len
        encoded_size += PermanentId::size(); // Perm ID
        encoded_size += self // Parent ID
            .parent_id
            .as_ref()
            .map_or(1, |_| 1 + PermanentId::size());
        encoded_size += ActorId::size(); //Owner ID
        encoded_size += 8usize * 2usize; // Created at and Modified at
        encoded_size += self.name.size(); //Node name

        encoded_size += 1; //metadata entry count
        encoded_size += self // Metadata
            .metadata()
            .iter()
            .map(|(k, v)| {
                2 // two  u8 lens for value and key
                + k.as_bytes().len() //key len
                 + v.len() //value len
            })
            .sum::<usize>();

        u64::try_from(encoded_size).expect("usize is larger than u64")
    }

    /// The owner of a node is the actor that created the specific version of this file. If a file
    /// is replaced or edited, the new actor will be the owner of the new version. Some client
    /// implementations make use of custom authorization middlewares that reject change violating
    /// more complex authorization policies such as only alllowing the owner to modify a file,
    /// enforce the actor is a member of a specific group, etc.
    pub fn owner_id(&self) -> ActorId {
        self.owner_id
    }

    /// If the node is a child in the overall filesystem, this will return the parent's permanent
    /// identifier. The only [`Node`] that does not have a parent is the root node within the
    /// filesystem.
    pub fn parent_id(&self) -> Option<PermanentId> {
        self.parent_id
    }

    #[tracing::instrument(skip(input))]
    pub(crate) fn parse<'a>(
        input: Stream<'a>,
        allocated_id: NodeId,
        data_key: Option<&AccessKey>,
    ) -> ParserResult<'a, Self> {
        tracing::trace!(allocated_id, "begin");

        let (input, cid) = Cid::parse(input)?;
        let (input, node_data_len) = le_u32.parse_peek(input)?;

        let node_data_start = input;

        let (input, permanent_id) = PermanentId::parse(input)?;
        let (input, vector_clock) = VectorClock::parse(input)?;
        let (input, parent_present) = take(1u8).parse_peek(input)?;

        tracing::trace!(node_data_len, ?cid, "cid/node_data_len");

        let (input, parent_id) = match parent_present[0] {
            0x00 => (input, None),
            0x01 => {
                let (node_data_buf, pid) = PermanentId::parse(input)?;
                (node_data_buf, Some(pid))
            }
            _ => {
                let err = winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Token,
                );
                return Err(winnow::error::ErrMode::Cut(err));
            }
        };

        let (input, owner_id) = ActorId::parse(input)?;
        let (input, created_at) = le_i64.parse_peek(input)?;
        let (input, modified_at) = le_i64.parse_peek(input)?;
        let (input, name) = NodeName::parse(input)?;
        let (mut input, metadata_entries) = le_u8.parse_peek(input)?;

        let mut metadata = HashMap::new();
        for _ in 0..metadata_entries {
            let (meta_buf, key_len) = le_u8.parse_peek(input)?;
            let (meta_buf, key) = take(key_len).parse_peek(meta_buf)?;
            let key_str = String::from_utf8(key.to_vec()).map_err(|_| {
                winnow::error::ErrMode::Cut(winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Token,
                ))
            })?;

            let (meta_buf, val_len) = le_u8.parse_peek(meta_buf)?;
            let (meta_buf, val) = take(val_len).parse_peek(meta_buf)?;
            let val = val.to_vec();

            metadata.insert(key_str, val);
            input = meta_buf;
        }

        let (input, inner) = NodeData::parse(input)?;
        debug_assert!(
            input.offset_from(&node_data_start) == usize::try_from(node_data_len).unwrap(), //Unwrap safe on 32bit and up systems (unsafe on 16 bit systems)
            "consumed to little or too much during parse based on the node's data_len field"
        );

        let node = Self {
            id: allocated_id,
            parent_id,
            permanent_id,
            owner_id,

            encoded_cache: EncodedCache::empty(),
            vector_clock,

            created_at,
            modified_at,

            name,
            metadata,

            inner,
        };

        Ok((input, node))
    }

    #[allow(dead_code)]
    pub(crate) async fn remove_child(
        &mut self,
        child_name: &NodeName,
    ) -> Result<(), NodeDataError> {
        self.inner.remove_child(child_name)?;
        self.notify_of_change().await;

        Ok(())
    }

    pub(crate) async fn remove_permanent_id(
        &mut self,
        child_id: &PermanentId,
    ) -> Result<(), NodeDataError> {
        self.inner.remove_permanent_id(child_id)?;
        self.notify_of_change().await;

        Ok(())
    }

    pub(crate) async fn set_name(&mut self, new_name: NodeName) {
        self.name = new_name;
        self.notify_of_change().await;
    }

    pub(crate) async fn set_parent_id(&mut self, parent_id: PermanentId) {
        self.parent_id = Some(parent_id);
        self.notify_of_change().await;
    }

    /// Returns the size of the node and all of its children (if they exist)
    pub fn size(&self) -> u64 {
        self.outer_size_estimate() + self.inner.size()
    }

    pub(crate) fn supports_children(&self) -> bool {
        matches!(self.inner.kind(), NodeKind::Directory | NodeKind::File)
    }

    /// Get Permanent Id of this Node
    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }

    /// Set attribute/metadata on this node
    pub async fn set_attribute(&mut self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        let old_value = self.metadata.insert(key, value);
        self.notify_of_change().await;
        old_value
    }

    async fn encoded<'a>(&'a self) -> std::io::Result<MappedRwLockReadGuard<'a, [u8]>> {
        let cached = self.encoded_cache.get();
        match cached {
            None => Ok(self.encoded_cache.set(self.compute_encoding().await?)),
            Some(inner) => Ok(inner),
        }
    }

    async fn compute_encoding(&self) -> std::io::Result<Vec<u8>> {
        let mut node_data = vec![0u8; Cid::size() + size_of::<u32>()]; // Reserve space for CID(32 bytes) and Length(4 bytes)

        self.permanent_id.encode(&mut node_data).await?;
        self.vector_clock.encode(&mut node_data).await?;

        match self.parent_id {
            Some(pid) => {
                node_data.write_all(&[0x01]).await?;
                pid.encode(&mut node_data).await?;
            }
            None => {
                node_data.write_all(&[0x00]).await?;
            }
        };

        self.owner_id.encode(&mut node_data).await?;

        let created_at_bytes = self.created_at.to_le_bytes();
        node_data.write_all(&created_at_bytes).await?;

        let modified_at_bytes = self.modified_at.to_le_bytes();
        node_data.write_all(&modified_at_bytes).await?;

        self.name.encode(&mut node_data).await?;

        let metadata_entry_count = u8::try_from(self.metadata.len())
            .map_err(|_| StdError::new(StdErrorKind::Other, "too many metadata entries"))?;

        node_data.write_all(&[metadata_entry_count]).await?;

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
        self.data().encode(&mut node_data).await?;

        //Set len
        let len = node_data.len() as u32 - 36;
        node_data[32..36].copy_from_slice(&len.to_le_bytes());

        // Set CID
        let cid = calculate_cid(&node_data[36..]);
        node_data[0..32].copy_from_slice(cid.as_bytes());
        

        Ok(node_data)
    }
}

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
            NodeData::AssociatedData { .. } => unimplemented!(),
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
