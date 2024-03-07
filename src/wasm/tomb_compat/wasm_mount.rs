use futures::io::Cursor;
use futures::StreamExt;
use uuid::Uuid;

use crate::prelude::*;

use js_sys::{Array, ArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::filesystem::{Drive, DriveLoader};
use crate::wasm::tomb_compat::{
    TombCompat, WasmBucket, WasmBucketMetadata, WasmFsMetadataEntry, WasmSnapshot,
};
use crate::wasm::utils::chacha_rng;

#[derive(Clone)]
#[wasm_bindgen]
pub struct WasmMount {
    wasm_client: TombCompat,

    bucket: WasmBucket,

    drive: Option<Drive>,
    dirty: bool,
}

impl WasmMount {
    pub(crate) async fn initialize(
        bucket: WasmBucket,
        wasm_client: TombCompat,
    ) -> BanyanFsResult<Self> {
        let mut rng = chacha_rng().map_err(|e| BanyanFsError::from(e.to_string()))?;
        let signing_key = wasm_client.signing_key();

        let api_assigned_id = bucket.id();
        let fs_uuid =
            Uuid::try_parse(&api_assigned_id).map_err(|e| BanyanFsError::from(e.to_string()))?;
        let filesystem_id = FilesystemId::from(fs_uuid.to_bytes_le());

        let drive = Drive::initialize_private_with_id(&mut rng, signing_key, filesystem_id)
            .map_err(|e| BanyanFsError::from(e.to_string()))?;

        tracing::warn!("impl needed: push initial metadata to the platform");

        let mount = Self {
            wasm_client,

            bucket,

            drive: Some(drive),
            dirty: true,
        };

        Ok(mount)
    }

    pub(crate) async fn pull(bucket: WasmBucket, wasm_client: TombCompat) -> BanyanFsResult<Self> {
        use platform::requests::metadata;

        let client = wasm_client.client();
        let bucket_id = bucket.id();

        let current_metadata = metadata::get_current(client, &bucket_id).await?;
        let metadata_id = current_metadata.id();

        // note(sstelfox): It doesn't make sense that we wouldn't have a signing key here, but if anything goes
        // wrong at this point we simply consider the drive to remain locked. There could be a 404
        // in here indicating that an initial metadata hasn't be pushed but that is a weird failure
        // case. We should really enforce an initial metadata push during the bucket creation...
        let drive = try_load_drive(client, &bucket_id, &metadata_id).await;
        let dirty = drive.is_none();

        let mount = Self {
            wasm_client,

            bucket,
            drive,
            dirty,
        };

        Ok(mount)
    }
}

#[wasm_bindgen]
impl WasmMount {
    // appears deprecated
    //pub async fn add(
    //    &mut self,
    //    _path_segments: js_sys::Array,
    //    _content_buffer: js_sys::ArrayBuffer,
    //) -> BanyanFsResult<()> {
    //    todo!()
    //}

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
        tracing::warn!("impl needed: not reporting snapshots as it hasn't been implemented yet");
        false
    }

    // checked
    pub fn locked(&self) -> bool {
        self.drive.is_none()
    }

    // checked, returns list of WasmFsMetadataEntry instances
    pub async fn ls(&mut self, path_segments: js_sys::Array) -> BanyanFsResult<js_sys::Array> {
        let path_segments = path_segments
            .iter()
            .map(|x| {
                x.as_string().ok_or(BanyanFsError::from(
                    "invalid path segment provided to wasm_mount#ls",
                ))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| BanyanFsError::from(e.to_string()))?;

        let unlocked_drive = match &self.drive {
            Some(drive) => drive,
            None => {
                return Err(BanyanFsError::from(
                    "unable to list directory contents of a locked bucket",
                ));
            }
        };

        let drive_root = unlocked_drive.root().await;

        let path_references = path_segments.iter().map(|x| x.as_str()).collect::<Vec<_>>();
        let entries = drive_root.ls(&path_references).await.map_err(|err| {
            BanyanFsError::from(format!(
                "error listing directory contents of {}: {}",
                path_segments.join("/"),
                err
            ))
        })?;

        let mut wasm_entries = Vec::new();
        for we in entries.into_iter() {
            let wasm_entry = WasmFsMetadataEntry::try_from(we)?;
            wasm_entries.push(wasm_entry);
        }

        Ok(vec_to_js_array(wasm_entries))
    }

    // checked
    pub async fn metadata(&self) -> BanyanFsResult<WasmBucketMetadata> {
        todo!()
    }

    // checked
    pub async fn mkdir(&mut self, _path_segments: js_sys::Array) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    pub async fn mv(
        &mut self,
        _from_path_segments: js_sys::Array,
        _to_path_segments: js_sys::Array,
    ) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    #[wasm_bindgen(js_name = readBytes)]
    pub async fn read_bytes(
        &mut self,
        _path_segments: js_sys::Array,
        _version: Option<String>,
    ) -> BanyanFsResult<Uint8Array> {
        todo!()
    }

    // checked
    #[wasm_bindgen]
    pub async fn remount(&mut self, _key_pem: String) -> BanyanFsResult<()> {
        tracing::warn!(
            "impl needed: remount, should be less necessary now but still should be implemented"
        );
        Ok(())
    }

    // checked
    pub async fn rename(&mut self, _name: String) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    pub async fn restore(&mut self, _wasm_snapshot: WasmSnapshot) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    pub async fn rm(&mut self, _path_segments: js_sys::Array) -> BanyanFsResult<()> {
        todo!()
    }

    // checked, returns URL to access file
    #[wasm_bindgen(js_name = shareFile)]
    pub async fn share_file(&mut self, _path_segments: js_sys::Array) -> BanyanFsResult<String> {
        todo!()
    }

    // checked
    #[wasm_bindgen(js_name = shareWith)]
    pub async fn share_with(&mut self, _bucket_key_id: String) -> BanyanFsResult<()> {
        todo!()
    }

    // checked
    #[wasm_bindgen(js_name = snapshot)]
    pub async fn snapshot(&mut self) -> BanyanFsResult<String> {
        todo!()
    }

    // checked
    pub async fn write(
        &mut self,
        _path_segments: Array,
        _content_buffer: ArrayBuffer,
    ) -> BanyanFsResult<()> {
        todo!()
    }
}

async fn try_load_drive(client: &ApiClient, bucket_id: &str, metadata_id: &str) -> Option<Drive> {
    use platform::requests::metadata;

    let key = client.signing_key()?;
    let mut stream = match metadata::pull_stream(client, bucket_id, metadata_id).await {
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
