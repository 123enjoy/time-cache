mod entity;
mod method;


use tokio::net::TcpStream;

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc};
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use method::{Exception};

type Db = Arc<Mutex<HashMap<String, method::TSQueue>>>;
// type Db = HashMap<u128, Bytes>;

struct TsConnection {
    socket: TcpStream,

}
pub async fn process(socket: &mut TcpStream, db: &Db) -> Result<(), Exception> {
    let mut buff = BytesMut::new();
    match parse_frame(socket, &mut buff).await {
        Ok(_) => {}
        Err(_e) => {
            return Err(Exception::new(-1, format!("Error while parsing frame from socket {}", _e).as_str()));
        }
    };
    if buff.is_empty() { return Ok(()); }
    let action = (&buff[0..2]).read_u16().await.unwrap();
    let param = &buff[6..];
    let m = method::choose_method(action);
    let mut map = db.lock().await;
    let mut out = BytesMut::new();
    let result = m.unwrap().do_method(&param, &mut map, &mut out);
    if result.is_err() {
        return Err(result.unwrap_err());
    }
    if out.len() != 0 {
        match socket.write_all(&out).await {
            Ok(_) => {}
            Err(_e) => {
                return Err(Exception::new(-1, "Error while writing to socket".to_string().as_str()));
            }
        }
    }
    Ok(())
}

async fn parse_frame(socket: &mut TcpStream, buff: &mut BytesMut) -> Result<(), Error> {
    let mut data = [0; 512];
    loop {
        if buff.len() > 6 {
            let length = (&buff[2..6]).read_u32().await.unwrap();
            let param = &buff[6..];
            if param.len() == length as usize {
                break;
            }
        }
        let n = match socket.read(&mut data).await {
            Ok(0) => {
                if buff.len() == 0 {
                    return Ok(());
                } else {
                    return Err(Error::from(ErrorKind::ConnectionReset));
                }
            }
            Ok(n) => n,
            Err(e) => {
                return Err(Error::new(ErrorKind::ConnectionReset, format!("connect reset error: {}", e)));
            }
        };
        buff.extend_from_slice(&data[..n]);
    }
    Ok(())
}

