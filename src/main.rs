#![allow(unused_imports)]

use std::io::{Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                tokio::spawn(async move {
                    let mut buffer = [0; 512];
                    loop {
                        match socket.read(&mut buffer).await {
                            Ok(0) => return,
                            Ok(_) => {
                                if let Err(e) = socket.write_all(b"+PONG\r\n").await {
                                    eprintln!("{:?}", e);
                                    return;
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
