use crate::entity::{TSCacheValue, TSItem, TSValue};
use crate::io::{read_all_items, write_all_items, FileIOCache};
use crate::method::{Exception, ExceptionKind, TSQueue};
use bytes::BytesMut;
use chrono::format::Item;
use serde_json::from_slice;
use std::arch::x86_64::_bextr_u64;
use std::collections::HashMap;
use std::mem;

pub struct CacheDb {
    cache: HashMap<String, TSQueue>,
    items: HashMap<String, TSItem>,
    ios: HashMap<String, FileIOCache>,
    data: Vec<BytesMut>,
    index: usize,
}

impl CacheDb {
    pub fn new() -> CacheDb {
        CacheDb {
            cache: HashMap::new(),
            items: HashMap::new(),
            ios: HashMap::new(),
            data: Vec::new(),
            index: 0,
        }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.cache.contains_key(key)
    }

    pub fn init(&mut self) {
        let mut values = vec![];
        read_all_items(&mut values);
        values.iter().for_each(|item| {
            self.create_item(item.clone());
        });
    }

    fn create_item(&mut self, item: TSItem) {
        let new = item.clone();
        let name = item.tsName;
        let cap = item.capacity;
        self.cache.insert(
            name.clone(),
            TSQueue::new(Box::new(new.clone()), cap as usize),
        );
        self.items.insert(name.clone(), new.clone());
        self.ios.insert(name, FileIOCache::new(Box::new(new)));
    }

    pub fn create_new_item(&mut self, item: TSItem, queue: TSQueue) {
        let new = item.clone();
        let new_item = item.clone();
        let name = item.tsName;
        self.cache.insert(name.clone(), queue);
        self.items.insert(name.clone(), new);
        self.ios.insert(name, FileIOCache::new(Box::new(new_item)));

        let mut values = vec![];
        self.items.iter().for_each(|(_, io)| {
            values.push(io);
        });
        write_all_items(&values);
    }

    pub fn insert_data(&mut self, data: BytesMut) -> usize {
        self.index += 1;
        self.data[self.index] = data;
        self.index
    }

    pub fn get_request_data(&self) -> &BytesMut {
        &self.data[self.index]
    }

    pub fn remove_data(&mut self) {
        self.data.remove(self.index);
        self.index -= 1;
    }

    pub fn insert_new_value<>(&mut self, ts_value: TSValue) -> Result<(), Exception>
    {

        if !self.contains_key(ts_value.name.as_str()) {
            return Err(Exception::err(
                ExceptionKind::TSNameExistsError,
                format!("TSName {} not exist", ts_value.name).as_str(),
            ));
        }
        let name = ts_value.name.clone();
        let item = self.items.get_mut(&name).unwrap();
        if !item.valueType.equal(&ts_value.value) {
            return Err(Exception::err(
                ExceptionKind::SaveTypeError,
                format!(
                    "except type:{:?},but input type:{:?}",
                    item.valueType, ts_value.value
                )
                .as_str(),
            ));
        }
        let queue = self.cache.get_mut(&name.clone()).unwrap();
        let io = self.ios.get_mut(&name).unwrap();
        io.append(&ts_value);
        // let cache = v.value;
        // let box_value = Box::new(cache);
        queue.insert(ts_value.key, ts_value.value.clone())
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut TSQueue> {
        self.cache.get_mut(key)
    }
}
