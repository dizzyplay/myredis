use crate::protocol::decoder::{RedisDecoder, RedisCommand};
use crate::protocol::encoder::RedisEncoder;
use crate::rdb::RDB;
use crate::store::Store;
use anyhow::Result;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use crate::config::Config;

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
                    Some(RedisCommand::ConfigGet(item)) => {
                        let key = item.to_uppercase();
                        match key.as_str() {
                            "DIR" | "DBFILENAME" => {
                                let config = match Config::new() {
                                    Ok(c) => {
                                        c
                                    },
                                    Err(e) => {
                                        eprintln!("Error: {:?}", e);
                                        encoder.encode_null(&mut response);
                                        continue;
                                    },
                                };
                                let value = match key.as_str() {
                                    "DIR" => config.dir.as_deref(),
                                    "DBFILENAME" => config.dbfilename.as_deref(),
                                    _ => unreachable!(),
                                };

                                if let Some(v) = value {
                                    let arr = [&key.as_str().to_lowercase(),v];
                                    encoder.encode_array(&mut response, &arr )
                                } else {
                                    encoder.encode_null(&mut response)
                                }
                            },
                            _ => {
                                encoder.encode_null(&mut response);
                                continue;
                            }
                        }
                    }
                    Some(RedisCommand::Ping) => {
                        encoder.encode_pong(&mut response);
                    }
                    Some(RedisCommand::Echo(message)) => {
                        encoder.encode_bulk_string(&mut response, &message);
                    }
                    Some(RedisCommand::Save) => {
                        match Config::new() {
                            Ok(config) => {
                                let dir = config.dir.unwrap_or_else(|| String::from("."));
                                let filename = config.dbfilename.unwrap_or_else(|| String::from("dump.rdb"));
                                let path = format!("{}/{}", dir, filename);
                                
                                match RDB::create_rdb(&path, Some(&[store.as_ref()])).await {
                                    Ok(_) => encoder.encode_ok(&mut response),
                                    Err(e) => {
                                        eprintln!("Failed to save RDB: {:?}", e);
                                        encoder.encode_error(&mut response);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to load config: {:?}", e);
                                encoder.encode_error(&mut response);
                            }
                        }
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
