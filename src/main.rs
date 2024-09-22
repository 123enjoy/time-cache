use tokio::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read};
use std::panic;
use std::sync::{Arc};
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, BufWriter};
use tokio::sync::Mutex;

#[path = "./lib.rs"]
mod lib;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let mut db = Arc::new(Mutex::new(HashMap::new()));
    println!("{:p}", &db);
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let db_ = db.clone();
        // println!("Accepted:{:p}",&db_);
        tokio::spawn(async move {
            loop {
               match lib::process(&mut socket, &db_).await{
                   Ok(_) => {},
                   Err(e) =>{
                       println!("{:?}", e);
                       if e.code == -1 {
                           break;
                       }
                   }
               };
            }
        });
    }
}


use tokio::io::{AsyncWriteExt};

#[tokio::main]
async fn main1() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            // In a loop, read data from the socket and write the data back.
            loop {
               match process(&mut socket).await{
                   Ok(_) => {},
                   Err(e) => {
                       println!("{:#?}", e);
                       break;
                   }
               };
            }
        });
    }
}


async fn process(socket: &mut TcpStream) -> Result<(),Error>{
    let mut  buff = vec![];
    let mut  steam = BufWriter::new(socket);
    loop {
        let mut buf = BytesMut::with_capacity(1);
        // steam.read_buf(&mut buf).await?;
        let n = match steam.read_buf(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                println!("read 0 bytes");
                return Err(Error::new(ErrorKind::Other, "read 0 bytes"));
            },
            Ok(n) => n,
            Err(e) => {
                eprintln!("failed to read from socket; err = {:?}", e);
                return Err(Error::new(ErrorKind::Other, "failed to read from socket; err = {:?}"));
            }
        };
        println!("read {} bytes", n);
        buff.extend_from_slice(&buf[..n]);
    }
    if buff.len() == 0 {
        println!("eof");
    }
    // Write the data back
   socket.write_all(&buff).await

}
