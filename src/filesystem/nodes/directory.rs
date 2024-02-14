use std::collections::HashMap;

use crate::codec::filesystem::DirectoryPermissions;
use crate::filesystem::NodeId;

#[derive(Clone, Debug, Default)]
pub struct Directory {
    permissions: DirectoryPermissions,
    children: HashMap<String, NodeId>,
    children_size: u64,
}

impl Directory {
    //pub fn ls(&self, path: &[&str]) -> Result<Vec<(String, DirectoryEntry)>, DirectoryError> {
    //    if path.is_empty() {
    //        let entries = self
    //            .children
    //            .iter()
    //            .map(|(k, v)| (k.clone(), v.clone()))
    //            .collect();

    //        return Ok(entries);
    //    }

    //    let (current_entry, next_path) = path.split_at(1);

    //    match self.children.get(current_entry[0]) {
    //        Some(DirectoryEntry::Directory(dir)) => dir.ls(next_path),
    //        Some(DirectoryEntry::File(_)) => Err(DirectoryError::NonDirectoryTraversal),
    //        None => Err(DirectoryError::NotFound),
    //    }
    //}

    //pub fn mkdir(
    //    &mut self,
    //    rng: &mut impl CryptoRngCore,
    //    owner: ActorId,
    //    path: &[&str],
    //    permissions: DirectoryPermissions,
    //    recursive: bool,
    //) -> Result<(), DirectoryError> {
    //    if path.is_empty() {
    //        return Err(DirectoryError::PathRequired);
    //    }

    //    let (name, next_path) = path.split_at(1);
    //    if next_path.is_empty() {
    //        let key = name[0].to_string();
    //        let mut dir = Directory::new(rng, owner);
    //        dir.permissions = permissions;

    //        self.children.insert(key, DirectoryEntry::Directory(dir));

    //        return Ok(());
    //    }

    //    match self.children.get_mut(name[0]) {
    //        Some(DirectoryEntry::Directory(dir)) => {
    //            dir.mkdir(rng, owner, next_path, permissions, recursive)?;
    //        }
    //        Some(DirectoryEntry::File(_)) => return Err(DirectoryError::NonDirectoryTraversal),
    //        None => {
    //            if !recursive {
    //                return Err(DirectoryError::NotFound);
    //            }

    //            let mut dir = Directory::new(rng, owner);
    //            dir.permissions = permissions;
    //            dir.mkdir(rng, owner, next_path, permissions, recursive)?;

    //            let key = name[0].to_string();
    //            self.children.insert(key, DirectoryEntry::Directory(dir));
    //        }
    //    }

    //    Ok(())
    //}

    pub fn new() -> Self {
        Directory {
            permissions: DirectoryPermissions::default(),
            children: HashMap::new(),
            children_size: 0,
        }
    }

    pub fn size(&self) -> u64 {
        self.children_size
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    #[error("path doesn't exist")]
    NotFound,

    #[error("path is not a directory")]
    NonDirectoryTraversal,

    #[error("empty paths are not allowed")]
    PathRequired,
}
