use tokio::net::TcpStream;

use crate::db::CacheDb;
use crate::entity::{DataType, SaveTimePeriod, TSCacheValue, TSItem, TSValue};
use crate::method::{choose_method, Exception, TSQueue};
use bytes::BytesMut;
use rmp_serde::to_vec_named;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

type Db = Arc<Mutex<CacheDb>>;
// type Db = HashMap<u128, Bytes>;
pub async fn process(socket: &mut TcpStream, db: &Db) -> Result<(), Exception> {
    let mut buff = BytesMut::new();
    match parse_frame(socket, &mut buff).await {
        Ok(_) => {}
        Err(_e) => {
            return Err(Exception::new(
                -1,
                format!("Error while parsing frame from socket {}", _e).as_str(),
            ));
        }
    };
    if buff.is_empty() {
        return Ok(());
    }
    let mut map = db.lock().await;
    // let index =  map.insert_data(buff);
    // let a = {
    //     let data = map.get_request_data();
    //   
    //     action
    // };
    let action = (&buff[0..2]).read_u16().await.unwrap();
    // let key = (&buff[2..4]).read_u16().await.unwrap();
    let m = choose_method(action);
    let mut out = BytesMut::new();
    let result = m.unwrap().do_method(&buff,&mut map, &mut out);
    if result.is_err() {
        return Err(result.unwrap_err());
    }
    if out.len() != 0 {
        let size = out.len();
        socket.write_u32((size) as u32).await.unwrap();
        match socket.write_all(&out).await {
            Ok(_) => {}
            Err(_e) => {
                return Err(Exception::new(
                    -1,
                    "Error while writing to socket".to_string().as_str(),
                ));
            }
        }
    } else {
        socket
            .write_all(
                to_vec_named(&TSCacheValue::String("OK".to_string()))
                    .unwrap()
                    .as_slice(),
            )
            .await
            .unwrap();
    }
    Ok(())
}

async fn parse_frame(socket: &mut TcpStream, buff: &mut BytesMut) -> Result<(), Error> {
    let length = socket.read_i32().await?;
    let mut data: Vec<u8> = vec![0; length as usize];
    socket.read_exact(&mut data).await?;
    buff.extend_from_slice(&data);
    Ok(())
}
