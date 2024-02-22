use std::collections::{HashMap, HashSet};
use std::io::{Error as StdError, ErrorKind as StdErrorKind};

use futures::io::AsyncWrite;
use slab::Slab;

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
        &self,
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
        //let mut data_ids = Vec::new();

        while let Some(node_id) = outstanding_ids.pop() {
            let node = self.nodes.get(node_id).ok_or_else(|| {
                StdError::new(StdErrorKind::Other, "node ID missing from internal nodes")
            })?;

            // Deduplicate nodes as we go through them
            let permanent_id = node.permanent_id();
            if seen_ids.contains(&permanent_id) {
                continue;
            }
            seen_ids.insert(permanent_id);

            let (written, children) = node.encode(writer, data_key).await?;

            for child_permanent_id in children.into_iter() {
                if seen_ids.contains(&child_permanent_id) {
                    continue;
                }

                let child_node_id = self.permanent_id_map.get(&permanent_id).ok_or_else(|| {
                    StdError::new(
                        StdErrorKind::Other,
                        "referenced child's permanent ID missing from internal nodes",
                    )
                })?;

                outstanding_ids.push(*child_node_id);
            }

            written_bytes += written;
        }

        Ok(written_bytes)
    }

    pub fn parse(_input: &[u8], _journal_start: JournalCheckpoint) -> ParserResult<Self> {
        todo!()
    }
}
