use super::dto::{DataEntry, DataEntryValue};
use crate::api_clients::node::dto::DataEntryResponse;

impl From<DataEntryResponse> for DataEntry {
    fn from(de: DataEntryResponse) -> Self {
        match de {
            DataEntryResponse::String { key, value } => DataEntry {
                key,
                value: DataEntryValue::String(value),
                address: String::default(),
            },
            DataEntryResponse::Integer { key, value } => DataEntry {
                key,
                value: DataEntryValue::Integer(value),
                address: String::default(),
            },
            DataEntryResponse::Boolean { key, value } => DataEntry {
                key,
                value: DataEntryValue::Boolean(value),
                address: String::default(),
            },
            DataEntryResponse::Binary { key, value } => DataEntry {
                key,
                value: DataEntryValue::Binary(value),
                address: String::default(),
            },
        }
    }
}
