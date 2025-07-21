use bytes::{BufMut, BytesMut};
use lazy_static::lazy_static;
use mini_redis::buffer;
use msgpack_simple::MsgPack;
use std::collections::HashMap;
use std::mem::take;
use tokio::sync::MutexGuard;

use crate::db::CacheDb;
use crate::entity::{DataType, TSCacheValue, TSItem, TSValue};
use crate::io::FileIOCache;
use crate::rscode::TSReturnCode::{TS0203, TS0301};
use rmp_serde::{from_slice, to_vec_named};
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

    pub unsafe fn query_times(&mut self, start_time: u64, end_time: u64) -> Vec<&TSCacheValue> {
        let mut buff = vec![];
        if self.len < self.capacity {
            unsafe {
                for i in 0..self.index {
                    if self.keys[i] < end_time && self.keys[i] > start_time {
                        buff.push(&*self.values[i].as_ref().unwrap())
                    }
                }
            }
        } else {
            for i in self.index..(self.index + self.capacity) {
                let mut j = i;
                if i >= self.capacity {
                    j = i % self.capacity;
                }
                if self.keys[j] < end_time && self.keys[j] > start_time {
                    buff.push(&*self.values[i].as_ref().unwrap())
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
    Set,
    Get,
}

impl MethodKind {
    pub fn as_code(&self) -> u16 {
        match self {
            MethodKind::Create => 301,
            MethodKind::Set => 501,
            MethodKind::Get => 601,
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
        out.extend_from_slice(header.as_ref());
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
                format!("TSName {} not exist", ts_name).as_str(),
            ));
        }
        let queue = db.get_mut(ts_name.as_str()).unwrap();
        let v = match queue.query_last() {
            Some(v) => v,
            None => {
                return Err(Exception::err(
                    ExceptionKind::QueueIsNullError,
                    format!("Queue is empty:{}", ts_name).as_str(),
                ))
            }
        };
        let ts_value = TSValue {
            name: ts_name,
            key: v.0,
            value: v.1.clone(),
        };
        out.extend_from_slice(header);
        out.extend_from_slice(&vec![DataType::Long.code(),v.1.clone().convert().code()]);
        out.extend_from_slice(to_vec_named(&ts_value).as_ref().unwrap());
        Ok(())
    }
}
