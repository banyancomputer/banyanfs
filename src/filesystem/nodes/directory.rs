use std::collections::HashMap;

use ecdsa::signature::rand_core::CryptoRngCore;
use rand::Rng;
use time::OffsetDateTime;

use crate::codec::filesystem::DirectoryPermissions;
use crate::codec::ActorId;
use crate::filesystem::nodes::file::File;

#[derive(Clone)]
pub struct Directory {
    #[allow(dead_code)]
    id: [u8; 16],
    owner: ActorId,

    permissions: DirectoryPermissions,
    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,

    metadata: HashMap<String, Vec<u8>>,
    children: HashMap<String, DirectoryEntry>,

    children_size: u64,
}

impl Directory {
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn get_attribute(&self, key: &str) -> Option<&[u8]> {
        self.metadata.get(key).map(Vec::as_slice)
    }

    pub fn ls(&self, path: &[&str]) -> Result<Vec<(String, DirectoryEntry)>, DirectoryError> {
        if path.is_empty() {
            let entries = self
                .children
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            return Ok(entries);
        }

        let (current_entry, next_path) = path.split_at(1);

        match self.children.get(current_entry[0]) {
            Some(DirectoryEntry::Directory(dir)) => dir.ls(next_path),
            Some(DirectoryEntry::File(_)) => Err(DirectoryError::NonDirectoryTraversal),
            None => Err(DirectoryError::NotFound),
        }
    }

    pub fn mkdir(
        &mut self,
        rng: &mut impl CryptoRngCore,
        owner: ActorId,
        path: &[&str],
        permissions: DirectoryPermissions,
        recursive: bool,
    ) -> Result<(), DirectoryError> {
        if path.is_empty() {
            return Err(DirectoryError::PathRequired);
        }

        let (name, next_path) = path.split_at(1);
        if next_path.is_empty() {
            let key = name[0].to_string();
            let mut dir = Directory::new(rng, owner);
            dir.permissions = permissions;

            self.children.insert(key, DirectoryEntry::Directory(dir));

            return Ok(());
        }

        match self.children.get_mut(name[0]) {
            Some(DirectoryEntry::Directory(dir)) => {
                dir.mkdir(rng, owner, next_path, permissions, recursive)?;
            }
            Some(DirectoryEntry::File(_)) => return Err(DirectoryError::NonDirectoryTraversal),
            None => {
                if !recursive {
                    return Err(DirectoryError::NotFound);
                }

                let mut dir = Directory::new(rng, owner);
                dir.permissions = permissions;
                dir.mkdir(rng, owner, next_path, permissions, recursive)?;

                let key = name[0].to_string();
                self.children.insert(key, DirectoryEntry::Directory(dir));
            }
        }

        Ok(())
    }

    pub fn modified_at(&self) -> OffsetDateTime {
        self.modified_at
    }

    pub fn new(rng: &mut impl CryptoRngCore, owner: ActorId) -> Self {
        let id: [u8; 16] = rng.gen();

        Directory {
            id,
            owner,

            permissions: DirectoryPermissions::default(),
            created_at: OffsetDateTime::now_utc(),
            modified_at: OffsetDateTime::now_utc(),

            metadata: HashMap::new(),
            children: HashMap::new(),

            children_size: 0,
        }
    }

    pub fn owner(&self) -> ActorId {
        self.owner
    }

    pub fn set_attribute(&mut self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        self.metadata.insert(key, value)
    }

    pub fn size(&self) -> u64 {
        self.children_size
    }
}

#[derive(Clone)]
pub enum DirectoryEntry {
    Directory(Directory),
    File(File),
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
