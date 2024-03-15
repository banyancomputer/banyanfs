use std::fmt::{Display as StdDisplay, Formatter as StdFormatter, Result as StdResult};

use wasm_bindgen::prelude::*;

use crate::codec::filesystem::NodeKind;
use crate::filesystem::nodes::NodeName;
use crate::filesystem::DirectoryEntry;

use super::BanyanFsError;

// todo: this had a lot of try from JS value traits, probably just being used as a stub type for
// parsing, this also didn't have wasm_bindgen before but I think its an improvement...
#[wasm_bindgen]
pub struct WasmFsMetadataEntry {
    name: String,
    entry_kind: String,
    metadata: JsValue,
}

#[wasm_bindgen]
impl WasmFsMetadataEntry {
    #[wasm_bindgen(getter = entry_type)]
    pub fn entry_kind(&self) -> String {
        tracing::trace!(?self.entry_kind, "entry_type getter called");
        self.entry_kind.clone()
    }

    #[wasm_bindgen(getter = metadata)]
    pub fn metadata(&self) -> JsValue {
        self.metadata.clone()
    }

    #[wasm_bindgen(getter = name)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl TryFrom<DirectoryEntry> for WasmFsMetadataEntry {
    type Error = WasmEntryError;

    fn try_from(dir_entry: DirectoryEntry) -> Result<Self, Self::Error> {
        let name = match dir_entry.name() {
            NodeName::Named(name) => name,
            _ => return Err("expected an entry name".into()),
        };

        let entry_kind = match dir_entry.kind() {
            NodeKind::File => "file",
            NodeKind::Directory => "directory",
            _ => return Err("unsupported entry kind".into()),
        };

        let metadata = js_sys::Object::new();

        let js_key = JsValue::from_str("created");
        let created_value = (dir_entry.created_at() / 1000) as u32;
        js_sys::Reflect::set(&metadata, &js_key, &created_value.into())
            .map_err(|_| "failed to convert created_at")?;

        let js_key = JsValue::from_str("modified");
        let modified_value = (dir_entry.modified_at() / 1000) as u32;
        js_sys::Reflect::set(&metadata, &js_key, &modified_value.into())
            .map_err(|_| "failed to convert modified_at")?;

        let js_key = JsValue::from_str("size");
        let size_value = dir_entry.size() as u32;
        js_sys::Reflect::set(&metadata, &js_key, &size_value.into())
            .map_err(|_| "failed to convert size")?;

        Ok(Self {
            name,
            entry_kind: entry_kind.to_string(),
            metadata: metadata.into(),
        })
    }
}

#[derive(Debug)]
pub struct WasmEntryError(String);

impl StdDisplay for WasmEntryError {
    fn fmt(&self, f: &mut StdFormatter<'_>) -> StdResult {
        write!(f, "encountered error with wasm directory entry: {}", self.0)
    }
}

impl std::error::Error for WasmEntryError {}

impl From<&'static str> for WasmEntryError {
    fn from(error: &'static str) -> Self {
        WasmEntryError(error.to_string())
    }
}

impl From<WasmEntryError> for BanyanFsError {
    fn from(error: WasmEntryError) -> Self {
        BanyanFsError(error.to_string())
    }
}
