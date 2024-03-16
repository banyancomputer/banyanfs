use serde::{Deserialize, Serialize};

use crate::api::platform::{ApiDrive, DriveKind, StorageClass};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct TombBucket {
    id: String,
    name: String,

    #[serde(rename = "type")]
    kind: DriveKind,

    storage_class: StorageClass,
}

impl TombBucket {
    pub(crate) fn from_components(
        id: String,
        name: String,
        storage_class: StorageClass,
        kind: DriveKind,
    ) -> Self {
        Self {
            id,
            name,
            kind,
            storage_class,
        }
    }

    pub(crate) fn id(&self) -> String {
        self.id.clone()
    }

    pub(crate) fn kind(&self) -> String {
        self.kind.to_string()
    }

    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }

    pub(crate) fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub(crate) fn storage_class(&self) -> String {
        self.storage_class.to_string()
    }
}

impl From<ApiDrive> for TombBucket {
    fn from(api_drive: ApiDrive) -> Self {
        Self {
            id: api_drive.id,
            name: api_drive.name,
            kind: api_drive.kind,
            storage_class: api_drive.storage_class,
        }
    }
}
