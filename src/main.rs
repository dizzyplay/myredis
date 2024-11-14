#![allow(unused_imports)]

mod encode;
mod store;

use crate::store::{Store};
use crate::encode::Encoder;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    let store: Arc<Store> = Arc::new(Store::new());
    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                let store_clone = Arc::clone(&store);
                tokio::spawn(async move {
                    let mut buffer = [0; 512];
                    loop {
                        match socket.read(&mut buffer).await {
                            Ok(0) => return,
                            Ok(bytes) => {
                                let s = &buffer[0..bytes];
                                let mut encoder = Encoder::new(s);
                                let mut result = encoder.parse();
                                while let Some(s) = result.pop_front() {
                                    match s.as_str() {
                                        "ECHO" => {
                                            let s = result.pop_front().unwrap();
                                            if let Err(e) = socket
                                                .write_all(
                                                    format!("${}\r\n{}\r\n", s.len(), s).as_bytes(),
                                                )
                                                .await
                                            {
                                                eprintln!("{:?}", e);
                                                return;
                                            }
                                        }
                                        "SET" => {
                                            let key = result.pop_front().unwrap();
                                            let value = result.pop_front().unwrap();
                                            store_clone.insert(key, value).await;
                                            if let Err(e) = socket
                                                .write_all(
                                                    format!(
                                                        "${}\r\n{}\r\n",
                                                        "OK".len(),
                                                        "OK"
                                                    )
                                                        .as_bytes(),
                                                )
                                                .await
                                            {
                                                eprintln!("{:?}", e);
                                                return;
                                            }
                                        }
                                        "GET" => {
                                            let key = result.pop_front().unwrap();
                                            let value = store_clone.get(&key);
                                            if let Some(value) = value.await {
                                                if let Err(e) = socket
                                                    .write_all(
                                                        format!(
                                                            "${}\r\n{}\r\n",
                                                            value.len(),
                                                            value
                                                        )
                                                            .as_bytes(),
                                                    )
                                                    .await
                                                {
                                                    eprintln!("{:?}", e);
                                                    return;
                                                }
                                            } else {
                                                if let Err(e) = socket.write_all(b"$-1\r\n").await {
                                                    eprintln!("{:?}", e);
                                                    return;
                                                }
                                            }
                                        }
                                        _ => {
                                            if let Err(e) = socket.write_all(b"+PONG\r\n").await {
                                                eprintln!("{:?}", e);
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("error reading from socket; err = {:?}", e);
                                return;
                            }
                        }
                    }
                });
            }
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}
