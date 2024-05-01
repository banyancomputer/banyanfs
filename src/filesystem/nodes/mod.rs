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

mod cid_cache;
mod node_builder;
mod node_data;
mod node_name;

pub(crate) use cid_cache::CidCache;
pub(crate) use node_builder::{NodeBuilder, NodeBuilderError};

pub(crate) use node_data::{NodeData, NodeDataError};
pub(crate) use node_name::{NodeName, NodeNameError};

use std::collections::HashMap;
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use futures::{AsyncWrite, AsyncWriteExt};
use winnow::binary::{le_i64, le_u32, le_u8};
use winnow::stream::Offset;
use winnow::token::take;
use winnow::Parser;

use crate::codec::crypto::AccessKey;
use crate::codec::filesystem::NodeKind;
use crate::codec::meta::{ActorId, Cid, PermanentId};
use crate::codec::{ParserResult, Stream, VectorClock};
use crate::filesystem::drive::OperationError;

pub(crate) type NodeId = usize;

/// The core structure that represents a node within the filesystem. This structure represents a
/// wrapper over any kind of filesystem metadata, structure, or associated data contained within
/// the BanyanFS structures.
///
/// ## Identifiers
///
/// When a [`Node`] instance needs to by referenced there are three relevant identifiers to be
/// aware of. When loaded into memory a [`Node`] will have a runtime identifier type of
/// [`NodeId`]. A [`NodeId`] should not be used for any reference that may need to persist
/// between serializations. A [`NodeId`] can also be reused if a node is deleted/removed and
/// and another one gets created.
///
/// A specific version of a Node should be referenced by the content identifier ([`Cid`]), this
/// takes into account all of the attributes that make up the filesystem entry. This is most useful
/// for permanently referencing the current version of a child node. Changes to children will also
/// trigger changes in the CID. The [`Cid`] of the root directory of a filesystem is used as the
/// version reference for the entire filesystem.
///
/// The final identifier is the [`PermanentId`]. This is used for journal entries, histories,
/// handles on other filesystem instances, parents of a current node, etc. When the handle you're
/// using needs to persist across versions this is the ID you'll want to use. Most internal
/// filesystem operations are based on these identifiers only finalizing the content into a [`Cid`]
/// when requested or when the filesystem is being written to permanent storage.
pub struct Node {
    id: NodeId,
    parent_id: Option<PermanentId>,
    permanent_id: PermanentId,
    owner_id: ActorId,

    cid: CidCache,
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
    /// hiearchy is directory -> (directory | file), file -> associated data.
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

    /// During the encoding process we first need the CID of the node (via [`Node::cid`]) which
    /// requires fully encoding the node to calculate. That method caches the result of that
    /// encoding if it needed to generate it. This consumes that cached encoding if we have it and
    /// is mostly used as an optimization that is described in the [`Node::cid`] documentation.
    #[allow(dead_code)]
    pub(crate) async fn cached_encoding(&self) -> Option<Vec<u8>> {
        self.cid.take_cached().await
    }

    /// Returns the CID of the node. If the internal data has changed in anyway (as indicated by
    /// and internal call to CidCache::is_dirty), this will fully encode the node as it would
    /// appear on disk and calculates the CID over that data.
    ///
    /// As an optimization this cache's that encoding so we don't have to re-encode it when we're
    /// writing the filesystem out to disk. This comes with a small memory penalty if some
    /// non-encoding process attempts to access a large number of node CIDs but that seems like an
    /// unlikely use case.
    pub async fn cid(&self) -> Result<Cid, OperationError> {
        if self.cid.is_dirty().await {
            let mut node_data = Vec::new();

            self.encode(&mut node_data).await.map_err(|_| {
                OperationError::InternalCorruption(self.id, "failed to encode node for CID")
            })?;

            self.cid.set_cached(node_data).await;
        }

        Ok(self.cid.cid().await.expect("enforced cid generation above"))
    }

    /// Returnes the unix timestamp (in milliseconds precision) of when the node was created.
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
        let mut node_data = Vec::new();

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

        self.data().encode(&mut node_data).await?;
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

        Ok(written_bytes)
    }

    /// Retrieves the in-memory identifier for this node instance, initialized when the node is
    /// first created from the drive that loads it. This is a volatile ID. See the
    /// [identifiers](#identifiers) section for detailed usage recommendations.
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

    async fn notify_of_change(&mut self) {
        self.cid.mark_dirty().await;
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
        let mut encoded_size = self
            .parent_id
            .as_ref()
            .map_or(1, |_| 1 + PermanentId::size() as u64);

        encoded_size += (Cid::size() + PermanentId::size() + ActorId::size() + 8 * 2) as u64;
        encoded_size += match self.name {
            NodeName::Root => 1,
            NodeName::Named(ref name) => 2 + name.as_bytes().len() as u64,
        };

        encoded_size += self
            .metadata()
            .iter()
            .map(|(k, v)| (2 + k.as_bytes().len() + v.len()) as u64)
            .sum::<u64>();

        encoded_size
    }

    /// The owner of a node is the actor that created the specific version of this file. If a file
    /// is replaced or edited, the new actor will be the owner of the new version. Some client
    /// implementations make use of custom authorization middlewares that reject change violating
    /// more complex authorization policies such as only allowing the owner to modify a file,
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

            cid: CidCache::from(cid),
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

    pub fn size(&self) -> u64 {
        self.outer_size_estimate() + self.inner.size()
    }

    pub(crate) fn supports_children(&self) -> bool {
        matches!(self.inner.kind(), NodeKind::Directory | NodeKind::File)
    }

    pub fn permanent_id(&self) -> PermanentId {
        self.permanent_id
    }

    pub async fn set_attribute(&mut self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        let old_value = self.metadata.insert(key, value);
        self.notify_of_change().await;
        old_value
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
