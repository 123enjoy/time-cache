use std::cmp::PartialEq;
use std::fmt::Formatter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error};
use crate::entity::TSCacheValue::Float;

trait TSMethod<T> {
    fn convert(self, bytes: &[u8]) -> T;
}
#[derive(Debug, Serialize, Deserialize, Clone,PartialEq)]
pub enum DataType {
    Float,
    Long,
    Double,
    Number,
    String,
    ByteArray,
}


impl DataType {
    pub fn length(&self) -> u16 {
        match self {
            DataType::Float => 4,
            DataType::Long => 8,
            DataType::Double => 8,
            DataType::Number => 8,
            DataType::String => 0,
            DataType::ByteArray => 0,
        }
    }
    pub fn equal(&self, value: &TSCacheValue) -> bool {
        match value {
            Float(_) => if let DataType::Float = self { true }else { false },
            TSCacheValue::Long(_) => if let DataType::Long = self { true }else { false },
            TSCacheValue::Double(_) => if let DataType::Double = self { true }else { false },
            TSCacheValue::Number(_) => if let DataType::Number = self { true }else { false },
            TSCacheValue::String(_) => if let DataType::String = self { true }else { false },
            TSCacheValue::ByteArray(_) => if let DataType::ByteArray = self { true }else { false },
        }
    }
}


#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SaveTimePeriod {
    Nerve,
    Minute,
    TenMinutes,
    Hour,
    Day,
}


impl SaveTimePeriod {
    pub fn as_period(&self) -> u128 {
        match self {
            SaveTimePeriod::Nerve => 0,
            SaveTimePeriod::Minute => 60,
            SaveTimePeriod::TenMinutes => 60 * 10,
            SaveTimePeriod::Hour => 3600,
            SaveTimePeriod::Day => 3600 * 24,
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TSItem {
    pub tsName: String,
    pub capacity: usize,
    pub datatype: DataType,
    pub saveTime: SaveTimePeriod,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct TSValue {
    pub name: String,
    pub key: u128,
    pub value: TSCacheValue,
}


#[derive(Debug, Clone,PartialEq)]
pub enum TSCacheValue {
    Float(f32),
    Long(i64),
    Double(f64),
    Number(f64),
    String(String),
    ByteArray(Vec<u8>),
}


impl Default for TSCacheValue {
    fn default() -> Self {
        Float(0.0)
    }
}
struct TSCacheValueVisitor;
impl<'de> serde::de::Visitor<'de> for TSCacheValueVisitor {
    type Value = TSCacheValue;
    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("error parse")
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> { Ok(TSCacheValue::Long(v)) }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> { Ok(TSCacheValue::Long(v as i64)) }
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E> { Ok(TSCacheValue::Float(v)) }
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> { Ok(TSCacheValue::Double(v)) }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> { Ok(TSCacheValue::String(v.to_owned())) }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> { Ok(TSCacheValue::String(v)) }
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> { Ok(TSCacheValue::ByteArray(v.to_owned())) }
}

impl<'de> Deserialize<'de> for TSCacheValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(TSCacheValueVisitor)
    }
}

impl Serialize for TSCacheValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TSCacheValue::Float(it) => serializer.serialize_f32(*it),
            TSCacheValue::Long(it) => serializer.serialize_i64(*it),
            TSCacheValue::Double(it) => serializer.serialize_f64(*it),
            TSCacheValue::Number(it) => serializer.serialize_f64(*it),
            TSCacheValue::String(it) => serializer.serialize_str(it),
            TSCacheValue::ByteArray(it) => serializer.serialize_bytes(it),
        }
    }
}
