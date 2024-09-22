use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::ptr::write;
use msgpack_simple::MsgPack;
use rmp_serde::{encode, from_slice, to_vec, to_vec_named};


#[path = "../src/entity.rs"]
mod entity;
#[path = "../src/method.rs"]
mod method;

use entity::{TSItem, DataType};
use crate::entity::TSCacheValue;

#[test]
fn test01() {
    let mut value = Some(34);
    let last = value.is_some_and(|x| {
        x == 33
    });
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
        datatype: DataType::Long,
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
    let demo = TSCacheValue::ByteArray(vec![1, 2, 3]);
    let encode_code = to_vec_named(&demo).unwrap();
    println!("encode len:{}", encode_code.len());
    println!("encode_code {:?}", encode_code);
    let ret: TSCacheValue = from_slice(&encode_code).unwrap();
    println!("{:#?}", ret);
}

