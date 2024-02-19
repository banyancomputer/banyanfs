use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveKind {
    Backup,
    Interactive,
}

impl Display for DriveKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DriveKind::Backup => write!(f, "backup"),
            DriveKind::Interactive => write!(f, "interactive"),
        }
    }
}

impl FromStr for DriveKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backup" => Ok(DriveKind::Backup),
            "interactive" => Ok(DriveKind::Interactive),
            _ => Err(format!("invalid bucket kind: {}", s)),
        }
    }
}
