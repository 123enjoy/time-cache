
use std::fs::{create_dir, create_dir_all, File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::time::SystemTime;
use byteorder::{BigEndian, WriteBytesExt};
use chrono::Local;
use rmp_serde::{to_vec_named,from_slice};
use crate::entity::{TSItem, TSValue, TSCacheValue, SaveTimePeriod};

static DATA: &str = "./data";


pub struct FileIOCache {
    ts_item: Box<TSItem>,
    path: String,
    write: Option<BufWriter<File>>,
    current_time: u128,
}

impl FileIOCache {
    pub fn new(ts_item: Box<TSItem>) -> FileIOCache {
        let mut io = FileIOCache {
            ts_item,
            path: "".to_string(),
            write: None,
            current_time: 0,
        };
        let item = &io.ts_item;
        io.path = format!("{}/{}", DATA, item.tsName);
        let path = Path::new(&io.path);
        if !path.exists() {
            create_dir_all(path).unwrap();
        }
        io
    }
    fn create_new_file(&mut self) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let dir = &format!("{}/{}", self.path, today);
        if !Path::new(&dir).exists() {
            create_dir_all(dir).unwrap()
        }
        let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
        if self.write.is_none() {
            self.current_time = time;
            self.write = Some(BufWriter::new(File::create(format!("{}/{}.tc", dir, time)).unwrap()));
        }
    }


    pub fn append(&mut self, value: &TSValue) {
        if self.ts_item.saveTime == SaveTimePeriod::Nerve {
            return;
        }
        if self.write.is_none() {
            self.create_new_file();
        }
        match self.write {
            Some(ref mut w) => {
                let mut buff = vec![];
                buff.write_u128::<BigEndian>(value.key).unwrap();
                buff.write(to_vec_named(&value.value).unwrap().as_slice()).unwrap();
                w.write_all(&buff).unwrap();
            }
            _ => {}
        }
        let period = self.ts_item.saveTime.as_period() * 1000;
        let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
        if (time - self.current_time) / period > 1 {
            self.close();
            self.write = None;
        }
    }

    pub fn close(&mut self) {
        match self.write {
            Some(ref mut w) => {
                w.flush().unwrap();
            }
            _ => {}
        }
    }
}


pub fn write_all_items(items: &Vec<&TSItem>) {
    let path = format!("{}/time-cache.tc", DATA);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .unwrap();
    file.write_all(to_vec_named(&items).unwrap().as_slice()).unwrap();
}


pub fn read_all_items(items: &mut Vec<TSItem>) {
    let path = format!("{}/time-cache.tc", DATA);
    let mut file = OpenOptions::new().read(true).open(path).unwrap();
    let mut buff = vec![];
    file.read_to_end(&mut buff).unwrap();
    let mut result = from_slice(&buff).unwrap();
    items.append(&mut result);
}


