use std::collections::{HashMap, HashSet};
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use ecdsa::signature::rand_core::CryptoRngCore;
use futures::io::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u64;
use slab::Slab;

use crate::codec::crypto::AccessKey;
use crate::codec::*;
use crate::filesystem::drive::DriveAccess;
use crate::filesystem::nodes::{Node, NodeId};

pub(crate) struct InnerDrive {
    pub(crate) access: DriveAccess,

    pub(crate) journal_start: JournalCheckpoint,
    pub(crate) root_node_id: NodeId,

    pub(crate) nodes: Slab<Node>,
    pub(crate) permanent_id_map: HashMap<PermanentId, NodeId>,
}

impl InnerDrive {
    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &mut self,
        rng: &mut impl CryptoRngCore,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut written_bytes = 0;

        // todo(sstelfox): there is a complex use case here that needs to be handled. Someone with
        // access to the filesystem and maintenance key, but without the data key can make changes
        // as long as they preserve the data key they loaded the filesystem with.
        //
        // Ideally data wrapping keys would be rotated everytime there was a full save but that won't work for
        // now. Both these cases can be iteratively added on later to the library.

        let data_key = self
            .access
            .permission_keys()
            .and_then(|pk| pk.data.as_ref())
            .ok_or(StdError::new(StdErrorKind::Other, "no data key"))?;

        // Walk the nodes starting from the root, encoding them one at a time, we want to make sure
        // we only encode things once and do so in a consistent order to ensure our content is
        // reproducible. This will silently discard any disconnected leaf nodes. Loops are
        // tolerated.

        let mut seen_ids = HashSet::new();
        let mut outstanding_ids = vec![self.root_node_id];
        let mut all_data_cids = Vec::new();

        let mut node_buffer = Vec::new();
        while let Some(node_id) = outstanding_ids.pop() {
            let node = self.nodes.get_mut(node_id).ok_or_else(|| {
                StdError::new(StdErrorKind::Other, "node ID missing from internal nodes")
            })?;

            // Deduplicate nodes as we go through them
            let permanent_id = node.permanent_id();
            if seen_ids.contains(&permanent_id) {
                continue;
            }
            seen_ids.insert(permanent_id);

            let (node_size, child_pids, data_cids) =
                node.encode(rng, &mut node_buffer, Some(data_key)).await?;

            let child_count = child_pids.as_ref().map_or(0, |p| p.len());
            let data_count = data_cids.as_ref().map_or(0, |d| d.len());

            let mut added_children = 0;
            if let Some(pid_list) = child_pids {
                for child_perm_id in pid_list.into_iter() {
                    if seen_ids.contains(&child_perm_id) {
                        continue;
                    }

                    let child_node_id =
                        self.permanent_id_map.get(&child_perm_id).ok_or_else(|| {
                            StdError::new(
                                StdErrorKind::Other,
                                "referenced child's permanent ID missing from internal nodes",
                            )
                        })?;

                    outstanding_ids.push(*child_node_id);
                    added_children += 1;
                }
            }

            tracing::trace!(?permanent_id, cid = ?node.cid(), node_kind = ?node.kind(), node_size, added_children, child_count, data_count, "node_encoding::complete");

            if let Some(data) = data_cids {
                all_data_cids.extend(data);
            }
        }

        // TODO: should scan the slab for any nodes that are not reachable from the root and track
        // them for removal in the journal and maintenance logs. It really shoudn't happen but be
        // defensive against errors...

        let root_node = &self.nodes[self.root_node_id];
        let root_perm_id = root_node.permanent_id();
        written_bytes += root_perm_id.encode(&mut node_buffer).await?;

        tracing::trace!(
            total_node_count = seen_ids.len(),
            ?root_perm_id,
            "node_encoding::complete"
        );

        let node_count = seen_ids.len() as u64;
        let node_count_bytes = node_count.to_le_bytes();
        writer.write_all(&node_count_bytes).await?;
        written_bytes += node_count_bytes.len();

        writer.write_all(&node_buffer).await?;
        written_bytes += node_buffer.len();

        Ok(written_bytes)
    }

    pub fn parse<'a>(
        input: &'a [u8],
        drive_access: DriveAccess,
        journal_start: JournalCheckpoint,
        data_key: Option<&AccessKey>,
    ) -> ParserResult<'a, Self> {
        let (input, root_perm_id) = PermanentId::parse(input)?;
        let (input, node_count) = le_u64(input)?;

        tracing::trace!(node_count, ?root_perm_id, "inner_drive::parse");

        let mut nodes = Slab::new();
        let mut permanent_id_map = HashMap::new();
        let mut expected_permanent_ids = HashSet::from([root_perm_id]);

        let mut node_input = input;
        for _ in 0..node_count {
            let entry = nodes.vacant_entry();
            let node_id = entry.key();

            let (remaining, node) = Node::parse(node_input, node_id, data_key)?;
            node_input = remaining;

            let permanent_id = node.permanent_id();

            if !expected_permanent_ids.contains(&permanent_id) {
                tracing::warn!(?permanent_id, ?node_id, node_kind = ?node.kind(), "found unexpected permanent ID in node data, skipping...");
                continue;
            }

            expected_permanent_ids.remove(&permanent_id);
            tracing::trace!(?permanent_id, ?node_id, node_kind = ?node.kind(), "inner_drive::parse::node");

            entry.insert(node);

            permanent_id_map.insert(permanent_id, node_id);
        }

        if !expected_permanent_ids.is_empty() {
            tracing::warn!(
                ?expected_permanent_ids,
                "missing expected permanent IDs in node data, fs missing data..."
            );
        }

        let root_node_id = *permanent_id_map.get(&root_perm_id).ok_or_else(|| {
            nom::Err::Failure(nom::error::make_error(
                node_input,
                nom::error::ErrorKind::Verify,
            ))
        })?;

        tracing::trace!(?root_node_id, "inner_drive::parse::complete");

        let inner_drive = InnerDrive {
            access: drive_access,
            journal_start,
            root_node_id,
            nodes,
            permanent_id_map,
        };

        Ok((node_input, inner_drive))
    }
}
