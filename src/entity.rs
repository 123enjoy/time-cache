use crate::entity::TSCacheValue::Float;
use lazy_static::lazy_static;
use serde::de::{Error, MapAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::cmp::PartialEq;
use std::fmt::{format, Formatter};
use std::io::Read;
use std::io::SeekFrom::Start;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

trait TSMethod<T> {
    fn convert(self, bytes: &[u8]) -> T;
}

static BYTE_ARRAYS_INDEX: AtomicUsize = AtomicUsize::new(0);

lazy_static! {
    static ref BYTE_ARRAYS_C: Vec<Box<[u8]>> = Vec::new();
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Float,
    INT,
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
            DataType::INT => 4,
            DataType::Number => 8,
            DataType::String => 0,
            DataType::ByteArray => 0,
        }
    }
    pub fn equal(&self, value: &TSCacheValue) -> bool {
        match value {
            Float(_) => DataType::Float == *self,
            TSCacheValue::Long(_) => DataType::Long == *self,
            TSCacheValue::Double(_) => DataType::Number == *self,
            TSCacheValue::Number(_) => DataType::Number == *self,
            TSCacheValue::String(_) => DataType::String == *self,
            TSCacheValue::ByteArray(_) => DataType::ByteArray == *self,
            TSCacheValue::INT(_) => DataType::INT == *self,
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            DataType::Float => 5,
            DataType::Long => 4,
            DataType::Double => 6,
            DataType::Number => 12,
            DataType::String => 8,
            DataType::ByteArray => 11,
            DataType::INT => 3,
        }
    }
}

pub fn data_type_match(value: &str) -> Result<DataType, String> {
    match value {
        "FLOAT" => Ok(DataType::Float),
        "LONG" => Ok(DataType::Long),
        "DOUBLE" => Ok(DataType::Double),
        "NUMBER" => Ok(DataType::Number),
        "STRING" => Ok(DataType::String),
        "byteARRAY" => Ok(DataType::ByteArray),
        _ => Err(format!("'{}' is not a valid type", value)),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveTimePeriod {
    NONE,
    Minute,
    TenMinutes,
    Hour,
    Day,
}

impl SaveTimePeriod {
    pub fn as_period(&self) -> u128 {
        match self {
            SaveTimePeriod::NONE => 0,
            SaveTimePeriod::Minute => 60,
            SaveTimePeriod::TenMinutes => 60 * 10,
            SaveTimePeriod::Hour => 3600,
            SaveTimePeriod::Day => 3600 * 24,
        }
    }
}

fn save_time_period_match(v: &str) -> Result<SaveTimePeriod, String> {
    match v {
        "NONE" => Ok(SaveTimePeriod::NONE),
        "MINITE" => Ok(SaveTimePeriod::Minute),
        "TENMINUTES" => Ok(SaveTimePeriod::TenMinutes),
        "HOUR" => Ok(SaveTimePeriod::Hour),
        "DAY" => Ok(SaveTimePeriod::Day),
        _ => Err(format!("'{}' is not a valid type", v)),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TSItem {
    pub tsName: String,
    pub capacity: u32,
    pub keyType: DataType,
    pub valueType: DataType,
    pub storageEnum: SaveTimePeriod,
}

#[derive(Debug, Serialize, Default)]
pub struct TSValue {
    pub name: String,
    pub key: u64,
    pub value: TSCacheValue,
}

impl<'a> Clone for TSValue {
    fn clone(&self) -> Self {
        TSValue {
            name: self.name.clone(),
            key: self.key,
            value: self.value.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum TSCacheValue {
    Float(f32),
    INT(i32),
    Long(i64),
    Double(f64),
    Number(f64),
    String(String),
    ByteArray(Arc<Box<Vec<u8>>>),
}

impl TSCacheValue {
    pub fn convert(self) -> DataType {
        match self {
            TSCacheValue::Float(_) => DataType::Float,
            TSCacheValue::INT(_) => DataType::INT,
            TSCacheValue::Long(_) => DataType::Long,
            TSCacheValue::Double(_) => DataType::Double,
            TSCacheValue::Number(_) => DataType::Number,
            TSCacheValue::String(_) => DataType::String,
            TSCacheValue::ByteArray(_) => DataType::ByteArray,
        }
    }
}

impl Default for TSCacheValue {
    fn default() -> Self {
        Float(0.0)
    }
}

struct DataTypeVisitor;
impl<'de> Visitor<'de> for DataTypeVisitor {
    type Value = DataType;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("error parse")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(data_type_match(v).unwrap())
    }
}

impl<'de> Deserialize<'de> for DataType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DataTypeVisitor)
    }
}

impl Serialize for DataType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DataType::Float => serializer.serialize_str("FLOAT"),
            DataType::Long => serializer.serialize_str("LONG"),
            DataType::INT => serializer.serialize_str("INT"),
            DataType::Double => serializer.serialize_str("DOUBLE"),
            DataType::Number => serializer.serialize_str("NUMBER"),
            DataType::String => serializer.serialize_str("STRING"),
            DataType::ByteArray => serializer.serialize_str("BYTEARRAY"),
        }
    }
}

struct SaveTimePeriodVisitor;
impl<'de> Visitor<'de> for SaveTimePeriodVisitor {
    type Value = SaveTimePeriod;
    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("error parse")
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(save_time_period_match(v).unwrap())
    }
}

impl<'de> Deserialize<'de> for SaveTimePeriod {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SaveTimePeriodVisitor)
    }
}

impl Serialize for SaveTimePeriod {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            SaveTimePeriod::NONE => serializer.serialize_str("NERVE"),
            SaveTimePeriod::Minute => serializer.serialize_str("MINUTE"),
            SaveTimePeriod::TenMinutes => serializer.serialize_str("TEN_MINUTES"),
            SaveTimePeriod::Hour => serializer.serialize_str("HOUR"),
            SaveTimePeriod::Day => serializer.serialize_str("DAY"),
        }
    }
}

struct TSCacheValueVisitor;
impl<'de> serde::de::Visitor<'de> for TSCacheValueVisitor {
    type Value = TSCacheValue;
    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("error parse")
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E> {
        Ok(TSCacheValue::INT(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
        Ok(TSCacheValue::Long(v))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
        Ok(TSCacheValue::Long(v as i64))
    }
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E> {
        Ok(TSCacheValue::Float(v))
    }
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
        Ok(TSCacheValue::Double(v))
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(TSCacheValue::String(v.to_owned()))
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(TSCacheValue::String(v))
    }

    // fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
    //     Ok(TSCacheValue::ByteArray()
    // }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(TSCacheValue::ByteArray(Arc::new(Box::new(v.to_vec()))))
    }
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
            TSCacheValue::ByteArray(it) => serializer.serialize_bytes(it.as_ref()),
            &TSCacheValue::INT(it) => serializer.serialize_i32(it),
        }
    }
}

struct TSValueVisitor;

impl<'de> serde::de::Visitor<'de> for TSValueVisitor {
    type Value = TSValue;
    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a ParsedData struct")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut name: Option<&'de str> = None;
        let mut value: Option<TSCacheValue> = None;
        let mut keys: Option<u64> = None;
        while let Some(key) = map.next_key()? {
            match key {
                "name" => {
                    name = map.next_value()?;
                }
                "value" => {
                    value = map.next_value()?;
                }
                "key" => {
                    keys = map.next_value()?;
                }
                _ => {
                    let _: () = map.next_value()?;
                }
            }
        }
        Ok(TSValue {
            name: name.unwrap().to_string(),
            key: keys.unwrap(),
            value: value.unwrap(),
        })
    }
}

impl<'de> Deserialize<'de> for TSValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("TSValue", &["name", "key", "value"], TSValueVisitor)
    }
}
