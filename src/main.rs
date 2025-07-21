mod db;
mod entity;
mod handle;
mod io;
mod method;
mod rscode;

use log::{info};
use rmp_serde::to_vec_named;
use std::sync::Arc;
use tokio::net::{TcpListener};
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;
use db::CacheDb;
#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let host = "0.0.0.0";
    let port = 7000;
    let addr = &format!("{}:{}", host, port);
    let listener = TcpListener::bind(addr).await.unwrap();
    let db = Arc::new(Mutex::new(CacheDb::new()));
    {
        let mut result = db.lock().await;
        result.init();
    }
    info!("Server started, listening on {}", addr);
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let db_ = db.clone();
        // println!("Accepted:{:p}",&db_);
        tokio::spawn(async move {
            loop {
                match handle::process(&mut socket, &db_).await {
                    Ok(_) => {}
                    Err(e) => {
                        info!("{:?}", e);
                        match socket.write_all(to_vec_named(&e).unwrap().as_slice()).await {
                            Ok(_) => {}
                            Err(e) => {
                                info!("{:?}", e);
                                break;
                            }
                        };
                        if e.code == -1 {
                            break;
                        }
                    }
                };
            }
        });
    }
}
