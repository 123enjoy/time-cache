use msgpack_simple::MsgPack;
use rmp_serde::{encode, from_slice, to_vec, to_vec_named};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;
use std::ptr::write;
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
    let mut ret = File::open("./msgpack_demo.out").unwrap();
    let mut buff = Vec::new();
    ret.read_to_end(&mut buff).unwrap();
    let result = MsgPack::parse(&*buff).unwrap().as_array().unwrap();
    println!("{:#?}", result);
}

#[test]
fn test05() {
    let demo = entity::TSItem {
        tsName: "demo".parse().unwrap(),
        capacity: 100,
        keyType: DataType::Long,
        valueType: DataType::Double,
        storageEnum: SaveTimePeriod::Nerve,
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
        storageEnum: SaveTimePeriod::Nerve,
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
        valueType: DataType::Double,
        storageEnum: SaveTimePeriod::Nerve,
    };
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
    // let mut s = [10, 40, 30];
    // let x = s.to_vec();
    // s[1] = 1;
    // println!("{:#?}", x);
    // println!("{:#?}", s);
}


fn demo(item: Box<TSItem>) {
    println!("{:p}", &*item);
    ok(item);
}

fn ok(item: Box<TSItem>) {
    println!("{:p}", &*item);
}
