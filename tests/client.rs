use std::fs::File;
use std::hash::RandomState;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::SystemTime;
use byteorder::{BigEndian, WriteBytesExt};
use msgpack_simple::MsgPack;
use rmp_serde::to_vec_named;
use rmp_serde::{from_slice};

#[path = "../src/entity.rs"]
mod entity;
#[path = "../src/method.rs"]
mod method;

#[path = "../src/io.rs"]
mod io;

#[path = "../src/db.rs"]
mod db;



use entity::{TSItem};
use crate::entity::*;
use crate::method::MethodKind;

#[test]
fn client_test() {
    let demo = TSItem {
        tsName: "demo".parse().unwrap(),
        capacity: 100,
        datatype: DataType::Long,
        saveTime: SaveTimePeriod::Minute,
    };
    let rt = serde_json::to_string(&demo).unwrap();
    println!("{}", rt);
    let encode_code = to_vec_named(&demo).unwrap();
    let mut buff = Vec::new();
    buff.write_u16::<BigEndian>(MethodKind::Create.as_code()).unwrap();
    buff.write_u32::<BigEndian>(encode_code.len() as u32).unwrap();
    buff.write_all(&encode_code).unwrap();
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.write_all(&buff).unwrap()
}

#[test]
fn client_test02() {
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    // let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    let value = TSValue {
        name: "demo".to_string(),
        key: time,
        value: TSCacheValue::Long(2),
    };
    let rt = serde_json::to_string(&value).unwrap();
    println!("{}", rt);

    let encode_code = to_vec_named(&value).unwrap();
    File::create("./val.out").unwrap().write_all(&encode_code).unwrap();

    let ret: TSValue = rmp_serde::from_slice(&encode_code).unwrap();
    println!("{:?}", ret);
    let mut buff = Vec::new();
    buff.write_u16::<BigEndian>(MethodKind::Set.as_code()).unwrap();
    buff.write_u32::<BigEndian>(encode_code.len() as u32).unwrap();
    buff.write_all(&encode_code).unwrap();

    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.write_all(&buff).unwrap();
    // let encode =

}

#[test]
fn client_test03() {
    // let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    let value = "demo";
    let rt = serde_json::to_string(&value).unwrap();
    println!("{}", rt);

    let encode_code = to_vec_named(&value).unwrap();
    let mut buff = Vec::new();
    buff.write_u16::<BigEndian>(MethodKind::Get.as_code()).unwrap();
    buff.write_u32::<BigEndian>(encode_code.len() as u32).unwrap();
    buff.write_all(&encode_code).unwrap();

    let str = MsgPack::parse(&encode_code).unwrap().as_string().unwrap();

    println!("{}", str);
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    stream.write_all(&buff).unwrap();

    let mut ret = vec![0u8; 1024];
    let n = stream.read(&mut ret).unwrap();
    let ret: TSCacheValue = from_slice(&ret[..n]).unwrap();
    println!("{:?}", value);
}


