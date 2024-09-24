use std::collections::HashMap;
use std::mem;
use chrono::format::Item;
use crate::entity::{TSCacheValue, TSItem, TSValue};
use crate::io::{read_all_items, write_all_items, FileIOCache};
use crate::method::{Exception, ExceptionKind, TSQueue};
pub struct CacheDb {
    cache: HashMap<String, TSQueue>,
    items: HashMap<String, TSItem>,
    ios: HashMap<String, FileIOCache>,
}

impl CacheDb {
    pub fn new() -> CacheDb {
        CacheDb { cache: HashMap::new(), items: HashMap::new(), ios: HashMap::new() }
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
        self.cache.insert(name.clone(), TSQueue::new(Box::new(new.clone()), cap));
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

    pub fn insert_new_value(&mut self, value: &mut TSValue) -> Result<(), Exception> {
        let mut v = mem::take(value);
        let item = self.items.get_mut(v.name.as_str()).unwrap();
        if !item.datatype.equal(&v.value) {
            return Err(Exception::err(ExceptionKind::SaveTypeError, format!("except type:{:?},but input type:{:?}", item.datatype, v.value).as_str()));
        }
        let io = self.ios.get_mut(v.name.as_str()).unwrap();
        io.append(&v);
        let queue = self.cache.get_mut(v.name.as_str()).unwrap();
        let cache = v.value;
        let box_value = Box::new(cache);
        queue.insert(v.key, box_value)
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut TSQueue> {
        self.cache.get_mut(key)
    }
}

