use std::collections::HashMap;
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
    metadata: HashMap<String, JsValue>,
}

#[wasm_bindgen]
impl WasmFsMetadataEntry {
    #[wasm_bindgen(getter = entry_type)]
    pub fn entry_kind(&self) -> String {
        self.entry_kind.clone()
    }

    #[wasm_bindgen(getter = metadata)]
    pub fn metadata(&self) -> JsValue {
        todo!("need to convert the disparate metadata to a js object")
    }

    #[wasm_bindgen(getter = name)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl TryFrom<DirectoryEntry> for WasmFsMetadataEntry {
    type Error = WasmEntryError;

    fn try_from(value: DirectoryEntry) -> Result<Self, Self::Error> {
        let name = match value.name() {
            NodeName::Named(name) => name,
            _ => return Err(WasmEntryError("expected an entry name".to_string())),
        };

        let entry_kind = match value.kind() {
            NodeKind::File => "file",
            NodeKind::Directory => "directory",
            _ => return Err(WasmEntryError("unsupported entry kind".to_string())),
        };

        tracing::warn!("not extracting metadata yet");
        let metadata = HashMap::new();

        Ok(Self {
            name,
            entry_kind: entry_kind.to_string(),
            metadata,
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

impl From<WasmEntryError> for BanyanFsError {
    fn from(error: WasmEntryError) -> Self {
        BanyanFsError(error.to_string())
    }
}
