use std::collections::HashSet;

use futures::io::Cursor;
use futures::StreamExt;

use crate::api::platform;
use crate::filesystem::Node;
use crate::prelude::*;

use js_sys::{Array, ArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::codec::Cid;
use crate::utils::crypto_rng;
use crate::wasm::tomb_compat::{
    TombCompat, WasmBucket, WasmBucketMetadata, WasmFsMetadataEntry, WasmSnapshot,
};
use crate::wasm::WasmDataStorage;

#[derive(Clone)]
#[wasm_bindgen]
pub struct WasmMount {
    wasm_client: TombCompat,

    bucket: WasmBucket,
    drive: Option<Drive>,
    store: WasmDataStorage,

    // Dirty should be a derived attribute based on the state of the drive and knowledge of the
    // state of the data cache.
    dirty: bool,
    last_saved_metadata: Option<WasmBucketMetadata>,
}

impl WasmMount {
    pub(crate) async fn initialize(
        bucket: WasmBucket,
        wasm_client: TombCompat,
        store: WasmDataStorage,
    ) -> BanyanFsResult<Self> {
        let mut rng = crypto_rng();
        let signing_key = wasm_client.signing_key();

        let api_assigned_id = bucket.id().replace("-", "");
        let mut id_bytes = [0u8; 16];

        for (i, byte_chunk) in api_assigned_id.as_bytes().chunks(2).enumerate() {
            let byte_str = std::str::from_utf8(byte_chunk).map_err(|_| {
                "UUID assigned from platform has non-hex and hyphen characters present"
            })?;

            let byte = u8::from_str_radix(byte_str, 16)
                .map_err(|_| "failed to convert byte string chunk to a byte")?;

            id_bytes[i] = byte;
        }

        let filesystem_id = FilesystemId::from(id_bytes);

        let drive = Drive::initialize_private_with_id(&mut rng, signing_key, filesystem_id)
            .map_err(|e| BanyanFsError::from(e.to_string()))?;

        let mut mount = Self {
            wasm_client,

            bucket,
            drive: Some(drive),
            store,

            dirty: true,
            last_saved_metadata: None,
        };

        mount.sync().await?;

        Ok(mount)
    }

    pub(crate) async fn pull(bucket: WasmBucket, wasm_client: TombCompat) -> BanyanFsResult<Self> {
        let client = wasm_client.client();
        let drive_id = bucket.id();

        let current_metadata = platform::metadata::get_current(client, &drive_id).await?;
        let metadata_id = current_metadata.id();

        // note(sstelfox): It doesn't make sense that we wouldn't have a signing key here, but if anything goes
        // wrong at this point we simply consider the drive to remain locked. There could be a 404
        // in here indicating that an initial metadata hasn't be pushed but that is a weird failure
        // case. We should really enforce an initial metadata push during the bucket creation...
        let drive = try_load_drive(client, &drive_id, &metadata_id).await;
        let dirty = drive.is_none();
        let store = wasm_client.store();

        let mount = Self {
            wasm_client,

            bucket,
            drive,
            store,

            dirty,

            last_saved_metadata: Some(WasmBucketMetadata::new(drive_id, current_metadata)),
        };

        Ok(mount)
    }

    pub(crate) async fn sync(&mut self) -> BanyanFsResult<()> {
        let mut rng = crypto_rng();

        let unlocked_drive = self
            .drive
            .as_ref()
            .ok_or(BanyanFsError::from("unable to sync locked bucket"))?;

        let content_options = ContentOptions::metadata();

        let mut encoded_drive = Vec::new();
        unlocked_drive
            .encode(&mut rng, content_options, &mut encoded_drive)
            .await
            .map_err(|e| format!("error while encoding drive for sync: {e}"))?;

        let expected_data_size = self
            .store
            .unsynced_data_size()
            .await
            .map_err(|e| format!("failed to read unsynced data size: {e}"))?;

        let root_cid = unlocked_drive
            .root_cid()
            .await
            .map_err(|e| format!("error while getting root cid for sync: {e}"))?;

        // todo(sstelfox): still need the following:
        let valid_keys = vec![];

        let deleted_block_cids = self
            .store
            .deleted_cids()
            .await
            .map_err(|e| format!("unable to retrieve deleted data CIDs: {e}"))?;

        let drive_stream = crate::api::client::utils::VecStream::new(encoded_drive).pinned();

        let push_response = platform::metadata::push_stream(
            self.wasm_client.client(),
            &self.bucket.id(),
            expected_data_size,
            root_cid,
            self.last_saved_metadata.as_ref().map(|m| m.id()).clone(),
            drive_stream,
            valid_keys,
            deleted_block_cids,
        )
        .await?;

        let new_metadata_id = push_response.id();
        tracing::info!(metadata_id = ?new_metadata_id, state = push_response.state(), "metadata recorded");

        if let Some(host) = push_response.storage_host() {
            if let Err(err) = self.store.set_sync_host(host.clone()).await {
                // In practice this should never happen, the trait defines an error type for
                // flexibility in the future but no implementations currently produce an error.
                tracing::warn!("failed to set sync host: {err}");
            }

            if let Some(grant) = push_response.storage_authorization() {
                self.wasm_client
                    .client()
                    .record_storage_grant(host, grant)
                    .await;
            }
        }

        let new_metadata = platform::metadata::get(
            self.wasm_client.client(),
            &self.bucket.id(),
            &new_metadata_id,
        )
        .await
        .map_err(|e| format!("error while fetching new metadata: {}", e))?;

        self.last_saved_metadata = Some(WasmBucketMetadata::new(self.bucket.id(), new_metadata));

        if let Err(err) = self.store.sync(&new_metadata_id).await {
            tracing::warn!("failed to sync data store to remotes, data remains cached locally but unsynced and can be retried: {err}");

            // note(sstelfox): this could be recoverable with future syncs, but we
            // should probably still fail here...
            return Err("failed to sync data store to remotes".into());
        }

        self.dirty = false;
        tracing::info!(metadata_id = &new_metadata_id, "drive synced");

        Ok(())
    }
}

#[wasm_bindgen]
impl WasmMount {
    // new, checked
    pub fn bucket(&self) -> WasmBucket {
        self.bucket.clone()
    }

    // checked
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    // checked
    #[wasm_bindgen(js_name = hasSnapshot)]
    pub fn has_snapshot(&self) -> bool {
        let metadata = match self.metadata() {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };

        metadata.api_metadata().snapshot_id().is_some()
    }

    // checked
    pub fn locked(&self) -> bool {
        self.drive.is_none()
    }

    // checked, returns list of WasmFsMetadataEntry instances
    pub async fn ls(&mut self, path_segments: js_sys::Array) -> BanyanFsResult<js_sys::Array> {
        let path_segments = path_segments
            .iter()
            .map(|x| x.as_string().ok_or("invalid path segments"))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| BanyanFsError::from(e.to_string()))?;

        let unlocked_drive = match &self.drive {
            Some(drive) => drive,
            None => return Err("unable to list directory contents of a locked bucket".into()),
        };

        let drive_root = unlocked_drive
            .root()
            .await
            .map_err(|_| "root unavailable")?;

        let path_refs = path_segments.iter().map(|x| x.as_str()).collect::<Vec<_>>();
        let entries = drive_root.ls(&path_refs).await.map_err(|err| {
            format!(
                "error listing directory contents of {}: {}",
                path_refs.join("/"),
                err
            )
        })?;

        let mut wasm_entries = Vec::new();
        for we in entries.into_iter() {
            match WasmFsMetadataEntry::try_from(we) {
                Ok(wasm_entry) => wasm_entries.push(wasm_entry),
                Err(err) => {
                    tracing::warn!("error converting fs entry (skipping): {}", err);
                }
            }
        }

        Ok(vec_to_js_array(wasm_entries))
    }

    // checked
    pub fn metadata(&self) -> BanyanFsResult<WasmBucketMetadata> {
        match &self.last_saved_metadata {
            Some(m) => Ok(m.clone()),
            None => Err("mount appears to be unsaved, no metadata available".into()),
        }
    }

    // checked
    pub async fn mkdir(&mut self, path_segments: js_sys::Array) -> BanyanFsResult<()> {
        let path_segments = parse_js_path(path_segments)?;
        let path_refs = path_segments.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        let unlocked_drive = match &self.drive {
            Some(drive) => drive,
            None => return Err("unable to create new directories in a locked bucket".into()),
        };

        let mut rng = crypto_rng();
        let mut drive_root = unlocked_drive
            .root()
            .await
            .map_err(|_| "root unavailable")?;

        drive_root
            .mkdir(&mut rng, path_refs.as_slice(), true)
            .await
            .map_err(|err| format!("error creating directory {}: {}", path_refs.join("/"), err))?;

        // note(sstelfox): ideally we don't need to sync after every change, but it doesn't seem
        // like there are any external checks currently to ensure changes are being written.
        self.dirty = true;
        self.sync().await?;

        Ok(())
    }

    // checked
    pub async fn mv(
        &mut self,
        src_path_segments: js_sys::Array,
        dst_path_segments: js_sys::Array,
    ) -> BanyanFsResult<()> {
        let src_path_segments = parse_js_path(src_path_segments)?;
        let src_path_refs = src_path_segments
            .iter()
            .map(|x| x.as_str())
            .collect::<Vec<_>>();

        let dst_path_segments = parse_js_path(dst_path_segments)?;
        let dst_path_refs = dst_path_segments
            .iter()
            .map(|x| x.as_str())
            .collect::<Vec<_>>();

        let unlocked_drive = match &self.drive {
            Some(drive) => drive,
            None => return Err("unable to move contents in a locked bucket".into()),
        };

        let mut rng = crypto_rng();
        let mut drive_root = unlocked_drive
            .root()
            .await
            .map_err(|_| "root unavailable")?;

        drive_root
            .mv(&mut rng, src_path_refs.as_slice(), dst_path_refs.as_slice())
            .await
            .map_err(|err| format!("error moving fs entry {}: {}", src_path_refs.join("/"), err))?;

        // note(sstelfox): ideally we don't need to sync after every change, but it doesn't seem
        // like there are any external checks currently to ensure changes are being written.
        self.dirty = true;
        self.sync().await?;

        Ok(())
    }

    // checked, version doesn't do anything
    #[wasm_bindgen(js_name = readBytes)]
    pub async fn read_bytes(
        &mut self,
        path_segments: js_sys::Array,
        _version: Option<String>,
    ) -> BanyanFsResult<Uint8Array> {
        let path_segments = parse_js_path(path_segments)?;
        let path_refs = path_segments.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        let unlocked_drive = match &self.drive {
            Some(drive) => drive,
            None => return Err("unable to delete content of a locked bucket".into()),
        };

        let drive_root = unlocked_drive
            .root()
            .await
            .map_err(|_| "root unavailable")?;

        let data = drive_root
            .read(&self.store, &path_refs)
            .await
            .map_err(|err| format!("failed to read data: {err:?}"))?;

        Ok(Uint8Array::from(data.as_slice()))
    }

    // checked
    #[wasm_bindgen]
    pub async fn remount(&mut self, _key_pem: String) -> BanyanFsResult<()> {
        tracing::warn!("impl might be needed: WasmMount#remount");
        Ok(())
    }

    // checked
    pub async fn rename(&mut self, name: String) -> BanyanFsResult<()> {
        let client = self.wasm_client.client();
        let drive_id = self.bucket.id();

        let update_drive_attrs = platform::ApiDriveUpdateAttributes {
            name: Some(name.clone()),
        };
        platform::drives::update(client, &drive_id, update_drive_attrs).await?;
        self.bucket.0.set_name(name);

        Ok(())
    }

    // checked
    pub async fn restore(&mut self, wasm_snapshot: WasmSnapshot) -> BanyanFsResult<()> {
        let client = self.wasm_client.client();
        let drive_id = self.bucket.id();
        let snapshot_id = wasm_snapshot.id();

        platform::snapshots::restore(client, &drive_id, &snapshot_id).await?;

        Ok(())
    }

    // checked
    pub async fn rm(&mut self, path_segments: js_sys::Array) -> BanyanFsResult<()> {
        let path_segments = parse_js_path(path_segments)?;
        let path_refs = path_segments.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        let unlocked_drive = match &self.drive {
            Some(drive) => drive,
            None => return Err("unable to delete content of a locked bucket".into()),
        };

        let mut drive_root = unlocked_drive
            .root()
            .await
            .map_err(|_| "root unavailable")?;

        drive_root
            .rm(&mut self.store, path_refs.as_slice())
            .await
            .map_err(|err| format!("error deleting fs entry {}: {}", path_refs.join("/"), err))?;

        // note(sstelfox): ideally we don't need to sync after every change, but it doesn't seem
        // like there are any external checks currently to ensure changes are being written.
        self.dirty = true;
        self.sync().await?;

        Ok(())
    }

    // checked, returns URL to access file
    #[wasm_bindgen(js_name = shareFile)]
    pub async fn share_file(&mut self, _path_segments: js_sys::Array) -> BanyanFsResult<String> {
        Err("share file is not currently implemented".into())
    }

    // checked
    #[wasm_bindgen(js_name = shareWith)]
    pub async fn share_with(&mut self, _bucket_key_id: String) -> BanyanFsResult<()> {
        Err("share with is not currently implemented".into())
    }

    // checked
    #[wasm_bindgen(js_name = snapshot)]
    pub async fn snapshot(&mut self) -> BanyanFsResult<String> {
        let current_metadata_id = match &self.last_saved_metadata {
            Some(metadata) => metadata.id(),
            None => return Err("unable to snapshot unsaved mount".into()),
        };

        let unlocked_drive = match &self.drive {
            Some(drive) => drive,
            None => return Err("unable to delete content of a locked bucket".into()),
        };

        // We need to get a list of all the data blocks involved in the current iteration of the
        // drive. We don't care about the order, and we want it deduplicated so we'll use a
        // HashSet.
        let mut data_block_cids = HashSet::new();

        fn get_node_data_cids(node: &Node) -> Result<Option<Vec<Cid>>, OperationError> {
            Ok(node.data_cids().clone())
        }

        let cid_groups = unlocked_drive
            .for_each_node(get_node_data_cids)
            .await
            .map_err(|e| format!("unable to extract data cids from drive: {e}"))?;

        for cid_group in cid_groups.into_iter() {
            for cid in cid_group.into_iter() {
                data_block_cids.insert(cid);
            }
        }

        let unique_block_cids = data_block_cids.into_iter().collect::<Vec<_>>();

        let snapshot_id = platform::snapshots::create(
            self.wasm_client.client(),
            &self.bucket.id(),
            &current_metadata_id,
            unique_block_cids.as_slice(),
        )
        .await?;

        Ok(snapshot_id)
    }

    // checked
    pub async fn write(
        &mut self,
        path_segments: Array,
        content_buffer: ArrayBuffer,
    ) -> BanyanFsResult<()> {
        let path_segments = parse_js_path(path_segments)?;
        let path_refs = path_segments.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        let unlocked_drive = match &self.drive {
            Some(drive) => drive,
            None => return Err("unable to delete content of a locked bucket".into()),
        };

        let mut rng = crypto_rng();
        let mut drive_root = unlocked_drive
            .root()
            .await
            .map_err(|_| "root unavailable")?;

        let file_data = Uint8Array::new(&content_buffer).to_vec();

        {
            if let Err(err) = drive_root
                .write(&mut rng, &mut self.store, &path_refs, &file_data)
                .await
            {
                let err_msg = format!("error writing to {}: {}", path_refs.join("/"), err);
                tracing::error!("{}", err_msg);
                return Err(err_msg.into());
            }
        }

        // note(sstelfox): ideally we don't need to sync after every change, but it doesn't seem
        // like there are any external checks currently to ensure changes are being written.
        self.dirty = true;
        self.sync().await?;

        Ok(())
    }
}

async fn try_load_drive(client: &ApiClient, drive_id: &str, metadata_id: &str) -> Option<Drive> {
    let key = client.signing_key();

    // todo(sstelfox): we should return something other than a 404 when we've seen at least once
    // metadata for a drive (if we've seen zero its safe to create a new drive, its not otherwise).
    let mut stream = match platform::metadata::pull_stream(client, drive_id, metadata_id).await {
        Ok(stream) => stream,
        Err(err) => {
            // note(sstelfox): there is a chance to dodge the API design issue mentioned in the
            // pull method, we may need to check if the response was a 404 and if so initialize a
            // new drive to return (as there may not have been one pushed previously).
            tracing::warn!("requested metadata unavailable: {}", err);
            return None;
        }
    };

    let mut drive_bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let byte_chunk = match chunk {
            Ok(byte_chunk) => byte_chunk,
            Err(err) => {
                tracing::warn!("error pulling chunk in metadata stream: {}", err);
                return None;
            }
        };

        drive_bytes.extend(byte_chunk.to_vec());
    }

    // todo(sstelfox): optimally we'd pass the Stream above directly to the loader rather than
    // loading it in memory. There are ways to do it but this is sufficient for the time being.

    let mut drive_cursor = Cursor::new(drive_bytes);
    let drive_loader = DriveLoader::new(&key);
    match drive_loader.from_reader(&mut drive_cursor).await {
        Ok(drive) => Some(drive),
        Err(err) => {
            tracing::warn!("error loading drive from metadata stream: {}", err);
            None
        }
    }
}

fn vec_to_js_array<T>(vec: Vec<T>) -> js_sys::Array
where
    T: Into<JsValue>,
{
    vec.into_iter().map(|x| x.into()).collect()
}

fn parse_js_path(path_arr: js_sys::Array) -> BanyanFsResult<Vec<String>> {
    let mut strings = Vec::new();

    for i in 0..path_arr.length() {
        let js_value = path_arr.get(i);

        let js_str = match js_value.dyn_into::<js_sys::JsString>() {
            Ok(js_str) => js_str,
            Err(_) => {
                return Err(BanyanFsError::from("non-string value present in path"));
            }
        };

        match js_str.as_string() {
            Some(string) => strings.push(string),
            None => return Err(BanyanFsError::from("invalid string present in path")),
        }
    }

    Ok(strings)
}
