use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageClass {
    Hot,
    Warm,
    Cold,
}

impl Display for StorageClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageClass::Hot => write!(f, "hot"),
            StorageClass::Warm => write!(f, "warm"),
            StorageClass::Cold => write!(f, "cold"),
        }
    }
}

impl FromStr for StorageClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hot" => Ok(StorageClass::Hot),
            "warm" => Ok(StorageClass::Warm),
            "cold" => Ok(StorageClass::Cold),
            _ => Err(format!("invalid storage class: {}", s)),
        }
    }
}
