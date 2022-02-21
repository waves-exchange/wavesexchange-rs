use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DataEntryValue {
    String(String),
    Integer(i64),
    Binary(Vec<u8>),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEntry {
    pub key: String,
    pub value: DataEntryValue,
    pub address: String,
}
