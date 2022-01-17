pub mod assets;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DataEntryValue {
    String(String),
    Integer(i64),
}
