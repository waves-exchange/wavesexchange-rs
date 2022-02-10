pub mod assets;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DataEntryValue {
    String(String),
    Integer(i64),
}
