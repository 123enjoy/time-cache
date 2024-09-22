use std::collections::HashMap;
use std::fmt::format;
use std::ops::Deref;
use bytes::{BufMut, BytesMut};
use lazy_static::lazy_static;
use msgpack_simple::MsgPack;
use tokio::sync::MutexGuard;
#[path = "./entity.rs"]
mod entity;

use rmp_serde::{from_slice, to_vec, to_vec_named};
use entity::{TSItem, TSValue, TSCacheValue};

pub struct TSQueue {
    ts_item: Box<TSItem>,
    capacity: usize,
    index: usize,
    len: usize,
    keys: Vec<u128>,
    values: Vec<Box<TSCacheValue>>,
}
impl TSQueue {
    pub fn new(item: Box<TSItem>, capacity: usize) -> TSQueue {
        TSQueue {
            ts_item: item,
            capacity,
            index: 0,
            len: 0,
            keys: vec![0; capacity],
            values: vec![Box::new(TSCacheValue::Long(0)); capacity],
        }
    }

    pub fn insert(&mut self, time: u128, value: Box<TSCacheValue>) {
        if self.index == self.capacity {
            self.index = 0;
        }
        if self.len == 0 && self.keys[0] >= time {
            return;
        }
        if self.len > 0 && self.index == 0 && self.keys[self.capacity - 1] >= time {
            return;
        }
        if self.len > 0 && self.index != 0 && self.keys[self.index - 1] >= time {
            return;
        }
        self.keys[self.index] = time;
        self.values[self.index] = value;
        self.index += 1;
        self.len += 1;
    }

    pub unsafe fn query_times(&mut self, start_time: u128, end_time: u128) -> Vec<&TSCacheValue> {
        let mut buff = vec![];
        if self.len < self.capacity {
            unsafe {
                for i in 0..self.index {
                    if self.keys[i] < end_time && self.keys[i] > start_time {
                        buff.push(&*self.values[i])
                    }
                }
            }
        } else {
            for i in self.index..(self.index + self.capacity) {
                let mut j = i;
                if i >= self.capacity { j = i % self.capacity; }
                if self.keys[j] < end_time && self.keys[j] > start_time {
                    buff.push(&*self.values[i])
                }
            }
        }
        buff
    }

    pub fn query_time(&mut self, time: u128) -> Option<&TSCacheValue> {
        let key = self.keys.iter().enumerate().min_by(|a, b| {
            (a.1 - time).cmp(&(b.1 - time))
        });
        if key.is_some() {
            return None;
        }
        let i = key?.0;
        // let cache_value = self.values;
        Some(&self.values[i])
    }

    pub fn query_last(&mut self) -> Option<&TSCacheValue> {
        Some(&self.values[self.index - 1])
    }
}

#[derive(Debug)]
pub struct Exception {
    pub code: i16,
    pub msg: String,
}

pub enum ExceptionKind {
    ParamParseError,
    TSNameExistsError,
    QueueIsNullError,
}

impl ExceptionKind {
    fn as_code(&self) -> i16 {
        match self {
            ExceptionKind::ParamParseError => 4001,
            ExceptionKind::TSNameExistsError => 4002,
            ExceptionKind::QueueIsNullError => 4003,
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
            MethodKind::Create => 101,
            MethodKind::Set => 201,
            MethodKind::Get => 301,
        }
    }
}

impl TSMethod {
    pub fn new(kind: MethodKind, method: Box<dyn Method>) -> TSMethod {
        TSMethod { code: kind.as_code(), method }
    }
}

lazy_static!(
    static ref  HANDLER_METHOD: Vec<TSMethod> = vec![
        TSMethod::new(MethodKind::Create,Box::new(CreateItemAction)),
        TSMethod::new(MethodKind::Set,Box::new(SetValueAction)),
        TSMethod::new(MethodKind::Get,Box::new(GetValueAction)),
    ];
);
pub fn choose_method(action: u16) -> Option<&'static Box<dyn Method>> {
    let methods: &Vec<TSMethod> = &*HANDLER_METHOD;
    methods.iter().find(|&method| { method.code == action }).map(|it| { &it.method })
}

pub trait Method: Send + Sync {
    fn do_method(&self, param: &[u8], db: &mut MutexGuard<HashMap<String, TSQueue>>, out: &mut BytesMut) -> Result<(), Exception>;
}
// #[derive(Debug, Copy,Clone)]
struct CreateItemAction;
impl Method for CreateItemAction {
    fn do_method(&self, param: &[u8], db: &mut MutexGuard<HashMap<String, TSQueue>>, out: &mut BytesMut) -> Result<(), Exception> {
        let item: TSItem = match from_slice(param) {
            Ok(v) => v,
            Err(e) => {
                return Err(Exception::err(ExceptionKind::ParamParseError, format!("parse msgpack error:{}", e).as_str()));
            }
        };
        if !db.contains_key(item.tsName.as_str()) {
            let new = item.clone();
            db.insert(item.tsName, TSQueue::new(Box::new(new), item.capacity));
        } else {
            return Err(Exception::err(ExceptionKind::TSNameExistsError, format!("duplicate TSName {}", item.tsName).as_str()));
        }
        Ok(())
    }
}

// Set
struct SetValueAction;
impl Method for SetValueAction {
    fn do_method(&self, param: &[u8], db: &mut MutexGuard<HashMap<String, TSQueue>>, out: &mut BytesMut) -> Result<(), Exception> {
        let value: TSValue = match from_slice(param) {
            Ok(v) => v,
            Err(e) => {
                return Err(Exception::err(ExceptionKind::ParamParseError, format!("parse msgpack error:{}", e).as_str()));
            }
        };
        if !db.contains_key(value.name.as_str()) {
            return Err(Exception::err(ExceptionKind::TSNameExistsError, format!("TSName {} not exist", value.name).as_str()));
        }
        let queue = db.get_mut(value.name.as_str()).unwrap();
        queue.insert(value.key, Box::new(value.value));
        Ok(())
    }
}


//
struct GetValueAction;
impl Method for GetValueAction {
    fn do_method(&self, param: &[u8], db: &mut MutexGuard<HashMap<String, TSQueue>>, out: &mut BytesMut) -> Result<(), Exception> {
        let ts_name = match MsgPack::parse(param) {
            Ok(v) => match v.as_string() {
                Ok(v) => v,
                Err(e) => {
                    return Err(Exception::err(ExceptionKind::ParamParseError, format!("parse msgpack error:{}", e).as_str()))
                }
            },
            Err(e) => {
                return Err(Exception::err(ExceptionKind::ParamParseError, format!("parse msgpack error:{}", e).as_str()))
            }
        };
        if !db.contains_key(ts_name.as_str()) {
            return Err(Exception::err(ExceptionKind::TSNameExistsError, format!("TSName {} not exist", ts_name).as_str()));
        }
        let queue = db.get_mut(ts_name.as_str()).unwrap();
        let v = match queue.query_last() {
            Some(v) => v,
            None => {
                return Err(Exception::err(ExceptionKind::QueueIsNullError, format!("Queue is empty:{}", ts_name).as_str()))
            }
        };
        out.put_slice(to_vec_named(v).unwrap().as_slice());
        Ok(())
    }
}








