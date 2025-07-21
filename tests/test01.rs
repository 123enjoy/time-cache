use msgpack_simple::MsgPack;
use rmp_serde::{encode, from_slice, to_vec, to_vec_named};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;
use std::ptr::write;
use std::sync::Arc;
use std::time::SystemTime;

#[path = "../src/entity.rs"]
mod entity;
#[path = "../src/method.rs"]
mod method;

#[path = "../src/io.rs"]
mod io;

#[path = "../src/db.rs"]
mod db;

#[path = "../src/rscode.rs"]
mod rscode;
use crate::entity::{SaveTimePeriod, TSCacheValue, TSValue};
use crate::io::read_all_items;
use entity::{DataType, TSItem};
use crate::entity::DataType::Float;

#[test]
fn test01() {
    let mut value = Some(34);
    let last = value.is_some_and(|x| x == 33);
    println!("last = {:?}", last);
}

#[test]
fn test02() {
    let mut ret = File::open("./msgpack_core.out").unwrap();
    let mut buff = Vec::new();
    ret.read_to_end(&mut buff).unwrap();
    let result = MsgPack::parse(&*buff).unwrap().as_map().unwrap();

    println!("{:#?}", result);
}

#[test]
fn test03() {
    let mut ret = File::open("./msgpack_demo.out").unwrap();
    let mut buff = Vec::new();
    ret.read_to_end(&mut buff).unwrap();
    let r: Vec<entity::TSValue> = from_slice(&buff).unwrap();
    println!("{:#?}", r);
    // let result = MsgPack::parse(&*buff).unwrap().as_array().unwrap();
    //
    // println!("{:#?}", result);
}

#[test]
fn test04() {
    let mut ret = File::open("./demo-1.bs").unwrap();
    let mut buff = Vec::new();
    ret.read_to_end(&mut buff).unwrap();
    // let result = MsgPack::parse(&*buff).unwrap().as_map().unwrap();
    let result : TSItem = from_slice(&buff).unwrap();
    println!("{:#?}", result);
}

#[test]
fn test05() {
    let demo = entity::TSItem {
        tsName: "demo".parse().unwrap(),
        capacity: 100,
        keyType: DataType::Long,
        valueType: DataType::Double,
        storageEnum: SaveTimePeriod::NONE,
    };
    let encode_code = to_vec_named(&demo).unwrap();
    println!("encode len:{}", encode_code.len());
    println!("encode_code {:?}", encode_code);
    let mut out = File::create("./demo.out").unwrap();
    out.write(&encode_code).expect("TODO: panic message");
    let ret: TSItem = from_slice(&encode_code).unwrap();
    println!("{:#?}", ret);
}

#[test]
fn test06() {
    // let data:[u8] = ;
    // let demo = TSCacheValue::ByteArray(vec![1, 2, 3]);
    // let encode_code = to_vec_named(&demo).unwrap();
    // println!("encode len:{}", encode_code.len());
    // println!("encode_code {:?}", encode_code);
    // let ret: TSCacheValue = from_slice(&encode_code).unwrap();
    // println!("{:#?}", ret);
}

#[test]
fn test07() {
    // let mut result = vec![];
    let item = vec![TSItem {
        tsName: "demo".parse().unwrap(),
        capacity: 100,
        keyType: DataType::Long,
        valueType: DataType::Double,
        storageEnum: SaveTimePeriod::NONE,
    }];
    let encode_code = to_vec_named(&item).unwrap();
    println!("encode len:{}", encode_code.len());
    println!("encode_code {:?}", encode_code);
    let ret: Vec<TSItem> = from_slice(&encode_code).unwrap();
    println!("{:#?}", ret);
    // read_all_items()
}

#[test]
fn test08() {
    let item = TSItem {
        tsName: "demo".parse().unwrap(),
        capacity: 100,
        keyType: DataType::Long,
        valueType: DataType::Number,
        storageEnum: SaveTimePeriod::NONE,
    };
    let bs = to_vec_named(&item).unwrap();

    println!("{:p}", &item);
    demo(Box::new(item))
}

#[test]
fn test09() {
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
    let value = TSValue {
        name: "demo".to_string(),
        key: time,
        value: TSCacheValue::Long(2),
    };
    let encode_code = to_vec_named(&value).unwrap();
    println!("{:#?}", encode_code);
}

#[test]
fn test10(){
    let cache = Arc::new(Box::new(vec![0x01,0x02]));
    let new_cache = cache.clone();
    println!("new_cache {:p}", new_cache);
    println!("cache {:p}", cache);
}

#[test]
fn test11(){
    let cache = &vec![0x01,0x02];
    let new_cache = &cache.to_vec()[..];
    println!("new_cache {:p}", new_cache.as_ptr());
    println!("cache {:p}", cache);
}

#[test]
fn test12(){
    let s = [1,2,3];
    let cache:Box<[i32]> = Box::new([0x01,0x02]);
    {
        println!("cache {:p}", &cache.as_ptr());
    }
    let new_cache = cache.into_vec();
    println!("new_cache {:p}", new_cache.as_ptr());
}



fn demo(item: Box<TSItem>) {
    println!("{:p}", &*item);
    ok(item);
}

fn ok(item: Box<TSItem>) {
    println!("{:p}", &*item);
}
