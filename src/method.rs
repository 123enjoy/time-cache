use bytes::{BufMut, BytesMut};
use lazy_static::lazy_static;
use log::warn;
use mini_redis::buffer;
use msgpack_simple::MsgPack;
use std::collections::HashMap;
use std::mem::take;
use tokio::sync::MutexGuard;

use crate::db::CacheDb;
use crate::entity::{DataType, TSCacheValue, TSItem, TSValue};
use crate::io::FileIOCache;
use crate::rscode::TSReturnCode;
use crate::rscode::TSReturnCode::{TS0203, TS0301};
use rmp_serde::{from_slice, to_vec, to_vec_named};
use serde::de::value;
use serde::{Deserialize, Serialize};
use ExceptionKind::{TSNameExistsError, TimeSerieError};

pub struct TSQueue {
    ts_item: Box<TSItem>,
    capacity: usize,
    index: usize,
    len: usize,
    keys: Vec<u64>,
    values: Vec<Option<TSCacheValue>>,
}
impl TSQueue {
    pub fn new(item: Box<TSItem>, capacity: usize) -> TSQueue {
        TSQueue {
            ts_item: item,
            capacity,
            index: 0,
            len: 0,
            keys: vec![0; capacity],
            values: vec![None; capacity],
        }
    }

    pub fn insert<'b>(&mut self, time: u64, value: TSCacheValue) -> Result<(), Exception> {
        if self.index == self.capacity {
            self.index = 0;
        }
        if self.len == 0 && self.keys[0] >= time {
            return Err(Exception::err(
                TimeSerieError,
                "time must be greater than 0",
            ));
        }
        if self.len > 0 && self.index == 0 && self.keys[self.capacity - 1] >= time {
            return Err(Exception::err(
                TimeSerieError,
                format!("current key:{} must be greater than last time", time).as_str(),
            ));
        }
        if self.len > 0 && self.index != 0 && self.keys[self.index - 1] >= time {
            return Err(Exception::err(
                TSNameExistsError,
                format!("current key:{} must be greater than last time", time).as_str(),
            ));
        }
        self.keys[self.index] = time;
        self.values[self.index] = Some(value);
        self.index += 1;
        self.len += 1;
        Ok(())
    }

    pub fn query_times(&mut self, start_time: u64, end_time: u64) -> Vec<TSValue> {
        let mut buff = vec![];
        if self.len < self.capacity {
            for i in 0..self.index {
                if self.keys[i] < end_time && self.keys[i] > start_time {
                    buff.push(TSValue{
                        name:self.ts_item.tsName.clone(),
                        key:self.keys[i],
                        value: self.values[i].as_ref().unwrap().clone()
                    })
                }
            }
        } else {
            for i in self.index..(self.index + self.capacity) {
                let mut j = i;
                if i >= self.capacity {
                    j = i % self.capacity;
                }
                if self.keys[j] < end_time && self.keys[j] > start_time {
                    buff.push(TSValue{
                        name:self.ts_item.tsName.clone(),
                        key:self.keys[i],
                        value: self.values[i].as_ref().unwrap().clone()
                    })
                }
            }
        }
        buff
    }

    pub fn query_time(&mut self, time: u64) -> Option<&TSCacheValue> {
        let key = self
            .keys
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - time).cmp(&(b.1 - time)));
        if key.is_some() {
            return None;
        }
        let i = key?.0;
        // let cache_value = self.values;
        let value = &self.values[i];
        value.as_ref()
    }

    pub fn query_last(&mut self) -> Option<(u64, &TSCacheValue)> {
        if self.len == 0 {
            None
        } else {
            Some((
                self.keys[self.index - 1],
                &self.values[self.index - 1].as_ref().unwrap(),
            ))
        }
    }

    pub fn get_value_type_code(&self) -> u8 {
        self.ts_item.valueType.code()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Exception {
    pub code: i16,
    pub msg: String,
}

pub enum ExceptionKind {
    ParamParseError,
    TSNameExistsError,
    QueueIsNullError,
    TimeSerieError,
    SaveTypeError,
}

impl ExceptionKind {
    fn as_code(&self) -> i16 {
        match self {
            ExceptionKind::ParamParseError => 4001,
            TSNameExistsError => 4002,
            ExceptionKind::QueueIsNullError => 4003,
            TimeSerieError => 4004,
            ExceptionKind::SaveTypeError => 4005,
        }
    }
}

impl Exception {
    pub fn new(code: i16, msg: &str) -> Exception {
        Exception {
            code,
            msg: msg.to_string(),
        }
    }

    pub fn err(kind: ExceptionKind, msg: &str) -> Exception {
        Exception::new(kind.as_code(), msg)
    }
    pub fn ok(&self, msg: &str) -> Exception {
        Exception::new(0, msg)
    }
}
pub struct TSMethod {
    code: u16,
    pub method: Box<dyn Method>,
}

pub enum MethodKind {
    Create,
    CreateMuti,
    EXIST,
    Set,
    SetMuti,

    Get,
    GetMuti,

    GetRange,
    GetRangeMuti,
}

impl MethodKind {
    pub fn as_code(&self) -> u16 {
        match self {
            MethodKind::Create => 301,     // 构建时间序列
            MethodKind::CreateMuti => 303, // 构建多个时间序列
            MethodKind::Set => 501,        // 设置一个最新值
            MethodKind::SetMuti => 505,    // 设置多个值

            MethodKind::Get => 601,
            MethodKind::GetMuti => 610,

            MethodKind::GetRange => 608,
            MethodKind::GetRangeMuti => 611,

            MethodKind::EXIST => 201,
        }
    }
}

impl TSMethod {
    pub fn new(kind: MethodKind, method: Box<dyn Method>) -> TSMethod {
        TSMethod {
            code: kind.as_code(),
            method,
        }
    }
}

lazy_static! {
    static ref HANDLER_METHOD: Vec<TSMethod> = vec![
        TSMethod::new(MethodKind::Create, Box::new(CreateItemAction)),
        TSMethod::new(MethodKind::Set, Box::new(SetValueAction)),
        TSMethod::new(MethodKind::Get, Box::new(GetValueAction)),
        TSMethod::new(MethodKind::EXIST, Box::new(ExistsAction)),
        TSMethod::new(MethodKind::GetMuti, Box::new(GetMutiAction)),
        TSMethod::new(MethodKind::SetMuti, Box::new(SetMutiAction)),
        TSMethod::new(MethodKind::CreateMuti, Box::new(CreateMutiAction)),
        TSMethod::new(MethodKind::GetRange, Box::new(GetRangeAction)),
         TSMethod::new(MethodKind::GetRangeMuti, Box::new(GetRangeMutiAction)),
    ];
}

lazy_static! {
    static ref FILE_CACHE: HashMap<String, FileIOCache> = {
        let h = HashMap::new();
        h
    };
}

pub fn choose_method(action: u16) -> Option<&'static Box<dyn Method>> {
    let methods: &Vec<TSMethod> = &*HANDLER_METHOD;
    methods
        .iter()
        .find(|&method| method.code == action)
        .map(|it| &it.method)
}

pub trait Method: Send + Sync {
    fn do_method(
        &self,
        buff: &BytesMut,
        db: &mut MutexGuard<CacheDb>,
        out: &mut BytesMut,
    ) -> Result<(), Exception>;
}
// #[derive(Debug, Copy,Clone)]
struct CreateItemAction;
impl Method for CreateItemAction {
    fn do_method(
        &self,
        buff: &BytesMut,
        db: &mut MutexGuard<CacheDb>,
        out: &mut BytesMut,
    ) -> Result<(), Exception> {
        let header = &buff[0..4];
        let param = &buff[4..];

        let item: TSItem = match from_slice(param) {
            Ok(v) => v,
            Err(e) => {
                return Err(Exception::err(
                    ExceptionKind::ParamParseError,
                    format!("parse msgpack error:{}", e).as_str(),
                ));
            }
        };
        out.extend_from_slice(header);
        let name = item.tsName.as_str();
        let cap = item.capacity;
        if !db.contains_key(name) {
            let new = item.clone();
            db.create_new_item(item, TSQueue::new(Box::new(new), cap as usize))
        }
        let code = TS0203.code() as i64;
        out.extend_from_slice(MsgPack::Int(code).encode().as_ref());
        Ok(())
    }
}

// Set
struct SetValueAction;
impl Method for SetValueAction {
    fn do_method(
        &self,
        buff: &BytesMut,
        db: &mut MutexGuard<CacheDb>,
        out: &mut BytesMut,
    ) -> Result<(), Exception> {
        let header = &buff[0..4];
        let param = &buff[4..];
        let value: TSValue = match from_slice(param) {
            Ok(v) => v,
            Err(e) => {
                return Err(Exception::err(
                    ExceptionKind::ParamParseError,
                    format!("parse msgpack error:{}", e).as_str(),
                ));
            }
        };
        out.extend_from_slice(header);
        let res = db.insert_new_value(value.clone());
        // buff.freeze();
        // out.extend_from_slice(header);
        let code = TS0301.code() as i64;
        out.extend_from_slice(MsgPack::Int(code).encode().as_ref());
        res
    }
}

//
struct GetValueAction;
impl Method for GetValueAction {
    fn do_method(
        &self,
        buff: &BytesMut,
        db: &mut MutexGuard<CacheDb>,
        out: &mut BytesMut,
    ) -> Result<(), Exception> {
        let header = &buff[0..2];
        let param = &buff[4..];
        let ts_name = match MsgPack::parse(param) {
            Ok(v) => match v.as_string() {
                Ok(v) => v,
                Err(e) => {
                    return Err(Exception::err(
                        ExceptionKind::ParamParseError,
                        format!("parse msgpack error:{}", e).as_str(),
                    ))
                }
            },
            Err(e) => {
                return Err(Exception::err(
                    ExceptionKind::ParamParseError,
                    format!("parse msgpack error:{}", e).as_str(),
                ))
            }
        };
        if !db.contains_key(ts_name.as_str()) {
            return Err(Exception::err(
                TSNameExistsError,
                &format!("TSName {} not exist", &ts_name),
            ));
        }
        let queue = db.get_mut(&ts_name).unwrap();
        let v = match queue.query_last() {
            Some(v) => v,
            None => {
                return Err(Exception::err(
                    ExceptionKind::QueueIsNullError,
                    &format!("Queue is empty:{}", &ts_name),
                ))
            }
        };
        let ts_value = TSValue {
            name: ts_name,
            key: v.0,
            value: v.1.clone(),
        };
        out.extend_from_slice(header);
        out.extend_from_slice(&vec![DataType::Long.code(), v.1.clone().convert().code()]);
        out.extend_from_slice(to_vec_named(&ts_value).as_ref().unwrap());
        Ok(())
    }
}

struct ExistsAction;
impl Method for ExistsAction {
    fn do_method(
        &self,
        buff: &BytesMut,
        db: &mut MutexGuard<CacheDb>,
        out: &mut BytesMut,
    ) -> Result<(), Exception> {
        let header = &buff[0..4];
        let param = &buff[4..];
        let ts_name = match MsgPack::parse(param) {
            Ok(v) => match v.as_string() {
                Ok(v) => v,
                Err(e) => {
                    return Err(Exception::err(
                        ExceptionKind::ParamParseError,
                        format!("parse msgpack error:{}", e).as_str(),
                    ))
                }
            },
            Err(e) => {
                return Err(Exception::err(
                    ExceptionKind::ParamParseError,
                    format!("parse msgpack error:{}", e).as_str(),
                ))
            }
        };

        let code = if !db.contains_key(ts_name.as_str()) {
            TSReturnCode::TS0201
        } else {
            TSReturnCode::TS0202
        };
        out.extend_from_slice(header);
        out.extend_from_slice(MsgPack::Int(code.code() as i64).encode().as_ref());
        Ok(())
    }
}

struct GetMutiAction;
impl Method for GetMutiAction {
    fn do_method(
        &self,
        buff: &BytesMut,
        db: &mut MutexGuard<CacheDb>,
        out: &mut BytesMut,
    ) -> Result<(), Exception> {
        let header = &buff[0..2];
        let param = &buff[4..];
        let ts_names: Vec<String> = match from_slice(param) {
            Ok(v) => v,
            Err(e) => {
                return Err(Exception::err(
                    ExceptionKind::ParamParseError,
                    format!("parse msgpack error:{}", e).as_str(),
                ))
            }
        };

        let mut ts_values = Vec::<TSValue>::new();
        let mut code: u8 = 0;
        for ts_name in ts_names {
            match db.get_mut(&ts_name) {
                Some(queue) => {
                    match queue.query_last() {
                        Some(v) => {
                            ts_values.push(TSValue {
                                name: ts_name,
                                key: v.0,
                                value: v.1.clone(),
                            });
                            code = queue.get_value_type_code();
                        }
                        None => {}
                    }

                    ()
                }
                _ => {
                    warn!("not exist key : {}", &ts_name);
                }
            };
        }
        out.extend_from_slice(header);
        out.extend_from_slice(&[DataType::Long.code(), code]);
        out.extend_from_slice(&to_vec_named(&ts_values).unwrap());

        Ok(())
    }
}

struct SetMutiAction;
impl Method for SetMutiAction {
    fn do_method(
        &self,
        buff: &BytesMut,
        db: &mut MutexGuard<CacheDb>,
        out: &mut BytesMut,
    ) -> Result<(), Exception> {
        let header = &buff[0..4];
        let param = &buff[4..];
        let ts_values: Vec<TSValue> = match from_slice(param) {
            Ok(v) => v,
            Err(e) => {
                return Err(Exception::err(
                    ExceptionKind::ParamParseError,
                    format!("parse msgpack error:{}", e).as_str(),
                ));
            }
        };

        for ts_value in ts_values {
            match db.insert_new_value(ts_value) {
                Err(e) => {
                    warn!("insert_new_value error: {}", e.msg);
                }
                _ => {}
            }
        }
        out.extend_from_slice(header);
        out.extend_from_slice(&MsgPack::Int(TS0301.code() as i64).encode());
        Ok(())
    }
}

struct CreateMutiAction;
impl Method for CreateMutiAction {
    fn do_method(
        &self,
        buff: &BytesMut,
        db: &mut MutexGuard<CacheDb>,
        out: &mut BytesMut,
    ) -> Result<(), Exception> {
        let header = &buff[0..4];
        let param = &buff[4..];
        let ts_items: Vec<TSItem> = from_slice(param).unwrap();
        for ts_item in ts_items {
            if !db.contains_key(&ts_item.tsName) {
                let cap = ts_item.capacity as usize;
                db.create_new_item(ts_item.clone(), TSQueue::new(Box::new(ts_item), cap))
            }
        }
        out.extend_from_slice(header);
        let code = TS0203.code() as i64;
        out.extend_from_slice(&MsgPack::Int(code).encode());
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TSKeyRange{
    name: String,
    begin: u64,
    end: u64,
}

struct GetRangeAction;
impl Method for GetRangeAction {
    fn do_method(&self, buff: &BytesMut, db: &mut MutexGuard<CacheDb>, out: &mut BytesMut) -> Result<(), Exception> {
        let header = &buff[0..2];
        let param = &buff[4..];
        let ts_range : TSKeyRange = from_slice(param).unwrap();
        let query = db.get_mut(&ts_range.name).unwrap();
        let result = query.query_times(ts_range.begin,ts_range.end);
        out.extend_from_slice(header);
        out.extend_from_slice(&[DataType::Long.code(),query.get_value_type_code()]);
        out.extend_from_slice(&to_vec_named(&result).unwrap());
        Ok(())
    }
}

struct GetRangeMutiAction;
impl Method for GetRangeMutiAction {
    fn do_method(&self, buff: &BytesMut, db: &mut MutexGuard<CacheDb>, out: &mut BytesMut) -> Result<(), Exception> {
        let header = &buff[0..2];
        let param = &buff[4..];
        let mut code:u8 = 0;
        let ts_ranges: Vec<TSKeyRange> = from_slice(param).unwrap();
        let mut result = vec![];
        for ts_range in ts_ranges {
            if db.contains_key(&ts_range.name) {
                let query = db.get_mut(&ts_range.name).unwrap();
                code = query.get_value_type_code();
                result.push(query.query_times(ts_range.begin,ts_range.end));
            }
        }
        out.extend_from_slice(header);
        out.extend_from_slice(&[DataType::Long.code(),code]);
        out.extend_from_slice(&to_vec_named(&result).unwrap());
        Ok(())
    }
}

