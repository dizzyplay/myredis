#![allow(unused_imports)]

mod command;
mod decode;
mod store;

use crate::decode::Decoder;
use crate::store::Store;
use bytes::BytesMut;
use command::process_command;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    let store: Arc<Store> = Arc::new(Store::new());
    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                let store_clone = Arc::clone(&store);
                tokio::spawn(async move {
                    loop {
                        let mut buf = BytesMut::with_capacity(512);
                        match socket.read_buf(&mut buf).await {
                            Ok(0) => {
                                println!("Connection closed");
                                return;
                            }
                            Ok(bytes) => {
                                println!("accepted {} bytes", bytes);
                                let s = buf.split_to(bytes);
                                match Decoder::new(s) {
                                    Ok(mut decoder) => match decoder.parse() {
                                        Ok(mut result) => {
                                            if let Ok(response) =
                                                process_command(&mut result, &store_clone).await
                                            {
                                                if let Err(e) =
                                                    socket.write_all(response.as_ref()).await
                                                {
                                                    eprintln!("{:?}", e);
                                                    return;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("{:?}", e);
                                            return;
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("error parsing command; err = {}", e);
                                        return;
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
