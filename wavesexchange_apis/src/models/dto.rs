use serde::{Deserialize, Serialize};

//TODO Most likely this `DataEntryValue` needs to be merged with `api_clients::node::dto::DataEntryResponse`

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DataEntryValue {
    String(String),
    Integer(i64),
    Binary(Vec<u8>),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataEntry {
    pub key: String,
    pub value: DataEntryValue,
    pub address: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[error("Wrong type of a value: {0} (expected {1})")]
pub struct TypeError(pub &'static str, pub &'static str);

impl DataEntryValue {
    #[inline]
    pub fn try_into_string(self) -> Result<String, TypeError> {
        match self {
            DataEntryValue::String(s) => Ok(s),
            _ => Err(self.type_error("String")),
        }
    }

    #[inline]
    pub fn try_as_str(&self) -> Result<&str, TypeError> {
        match self {
            DataEntryValue::String(s) => Ok(s.as_str()),
            _ => Err(self.type_error("String")),
        }
    }

    #[inline]
    pub fn try_into_integer(self) -> Result<i64, TypeError> {
        match self {
            DataEntryValue::Integer(i) => Ok(i),
            _ => Err(self.type_error("Integer")),
        }
    }

    #[inline]
    pub fn try_as_integer(&self) -> Result<i64, TypeError> {
        match self {
            DataEntryValue::Integer(i) => Ok(*i),
            _ => Err(self.type_error("Integer")),
        }
    }

    #[inline]
    pub fn try_into_binary(self) -> Result<Vec<u8>, TypeError> {
        match self {
            DataEntryValue::Binary(b) => Ok(b),
            _ => Err(self.type_error("Binary")),
        }
    }

    #[inline]
    pub fn try_as_binary(&self) -> Result<&[u8], TypeError> {
        match self {
            DataEntryValue::Binary(b) => Ok(b.as_slice()),
            _ => Err(self.type_error("Binary")),
        }
    }

    #[inline]
    pub fn try_into_boolean(self) -> Result<bool, TypeError> {
        match self {
            DataEntryValue::Boolean(b) => Ok(b),
            _ => Err(self.type_error("Boolean")),
        }
    }

    #[inline]
    pub fn try_as_boolean(&self) -> Result<bool, TypeError> {
        match self {
            DataEntryValue::Boolean(b) => Ok(*b),
            _ => Err(self.type_error("Boolean")),
        }
    }

    #[inline]
    pub fn value_type_name(&self) -> &'static str {
        match self {
            DataEntryValue::String(_) => "String",
            DataEntryValue::Integer(_) => "Integer",
            DataEntryValue::Binary(_) => "Binary",
            DataEntryValue::Boolean(_) => "Boolean",
        }
    }

    #[inline]
    fn type_error(&self, expected: &'static str) -> TypeError {
        TypeError(self.value_type_name(), expected)
    }
}

impl TryFrom<DataEntryValue> for String {
    type Error = TypeError;

    #[inline]
    fn try_from(value: DataEntryValue) -> Result<Self, Self::Error> {
        match value {
            DataEntryValue::String(s) => Ok(s),
            _ => Err(value.type_error("String")),
        }
    }
}

impl TryFrom<DataEntryValue> for i64 {
    type Error = TypeError;

    #[inline]
    fn try_from(value: DataEntryValue) -> Result<Self, Self::Error> {
        match value {
            DataEntryValue::Integer(i) => Ok(i),
            _ => Err(value.type_error("Integer")),
        }
    }
}

impl TryFrom<DataEntryValue> for Vec<u8> {
    type Error = TypeError;

    #[inline]
    fn try_from(value: DataEntryValue) -> Result<Self, Self::Error> {
        match value {
            DataEntryValue::Binary(b) => Ok(b),
            _ => Err(value.type_error("Binary")),
        }
    }
}

impl TryFrom<DataEntryValue> for bool {
    type Error = TypeError;

    #[inline]
    fn try_from(value: DataEntryValue) -> Result<Self, Self::Error> {
        match value {
            DataEntryValue::Boolean(b) => Ok(b),
            _ => Err(value.type_error("Boolean")),
        }
    }
}

#[test]
fn test_data_entry_type_conversions() {
    let v = DataEntryValue::String("test".to_string());
    assert_eq!(v.try_as_str(), Ok("test"));
    assert!(v.try_as_integer().is_err());
    assert!(v.try_as_binary().is_err());
    assert!(v.try_as_boolean().is_err());
    assert_eq!(String::try_from(v), Ok("test".to_string()));

    let v = DataEntryValue::Integer(42);
    assert_eq!(v.try_as_integer(), Ok(42));
    assert!(v.try_as_str().is_err());
    assert!(v.try_as_binary().is_err());
    assert!(v.try_as_boolean().is_err());
    assert_eq!(i64::try_from(v), Ok(42));

    let v = DataEntryValue::Binary(vec![0xAA, 0xBB]);
    assert_eq!(v.try_as_binary(), Ok([0xAA_u8, 0xBB_u8].as_slice()));
    assert!(v.try_as_str().is_err());
    assert!(v.try_as_integer().is_err());
    assert!(v.try_as_boolean().is_err());
    assert_eq!(Vec::try_from(v), Ok(vec![0xAA, 0xBB]));

    let v = DataEntryValue::Boolean(true);
    assert_eq!(v.try_as_boolean(), Ok(true));
    assert!(v.try_as_str().is_err());
    assert!(v.try_as_integer().is_err());
    assert!(v.try_as_binary().is_err());
    assert_eq!(bool::try_from(v), Ok(true));
}
