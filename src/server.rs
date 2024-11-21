use crate::protocol::decoder::{RedisDecoder, RedisCommand};
use crate::protocol::encoder::RedisEncoder;
use crate::store::Store;
use anyhow::Result;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct Server {
    listener: TcpListener,
    store: Arc<Store>,
}

impl Server {
    pub async fn new(addr: &str) -> Result<Server> {
        let listener = TcpListener::bind(addr).await?;
        let store = Arc::new(Store::new());

        Ok(Server { listener, store })
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            let (socket, _) = self.listener.accept().await?;
            let store = Arc::clone(&self.store);

            tokio::spawn(async move {
                if let Err(err) = handle_connection(socket, store).await {
                    eprintln!("Error: {:?}", err);
                }
            });
        }
    }
}

async fn handle_connection(mut socket: TcpStream, store: Arc<Store>) -> Result<()> {
    let mut buf = BytesMut::with_capacity(1024);
    let decoder = RedisDecoder::new();
    let encoder = RedisEncoder::new();
    let mut response = BytesMut::new();

    loop {
        match socket.read_buf(&mut buf).await? {
            0 => break, // connection closed
            bytes => {
                println!("accepted {} bytes", bytes);
                let mut s = buf.split_to(bytes);

                match decoder.decode(&mut s) {
                    Some(RedisCommand::Set(key, value, expiry)) => {
                        store.insert(key, value, expiry).await;
                        encoder.encode_ok(&mut response);
                    }
                    Some(RedisCommand::Get(key)) => {
                        match store.get(&key).await {
                            Some(value) => encoder.encode_bulk_string(&mut response, &value),
                            None => encoder.encode_null(&mut response),
                        }
                    }
                    Some(RedisCommand::ConfigGet(_)) => {
                        encoder.encode_null(&mut response);
                    }
                    Some(RedisCommand::Ping) => {
                        encoder.encode_pong(&mut response);
                    }
                    Some(RedisCommand::Echo(message)) => {
                        encoder.encode_bulk_string(&mut response, &message);
                    }
                    Some(RedisCommand::Unknown) => {
                        encoder.encode_error(&mut response);
                    }
                    None => {
                        encoder.encode_error(&mut response);
                    }
                };

                socket.write_all(&response).await?;
                response.clear();
            }
        }
    }
    Ok(())
}
