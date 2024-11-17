use crate::command::process_command;
use crate::decode::Decoder;
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
    pub async fn new(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let store = Arc::new(Store::new());

        Ok(Server { listener, store })
    }

    pub async fn run(&self) -> Result<()> {
        println!("Server running on {}", self.listener.local_addr()?);

        loop {
            let (socket, addr) = self.listener.accept().await?;
            println!("Accepted connection from: {}", addr);

            let store = Arc::clone(&self.store);
            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, store).await {
                    eprintln!("Error handling connection: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(mut socket: TcpStream, store: Arc<Store>) -> Result<()> {
    loop {
        let mut buf = BytesMut::with_capacity(512);
        match socket.read_buf(&mut buf).await? {
            0 => {
                println!("Connection closed");
                return Ok(());
            }
            bytes => {
                println!("accepted {} bytes", bytes);
                let s = buf.split_to(bytes);
                if let Ok(mut result) = Decoder::new(s) {
                    if let Ok(mut command_list) = result.parse() {
                        if let Ok(response) = process_command(&mut command_list, &store).await {
                            socket.write_all(response.as_ref()).await?;
                        } else if let Err(e) = process_command(&mut command_list, &store).await {
                            eprintln!("Error processing command: {}", e);
                            socket.write_all(b"-ERR\r\n").await?;
                        }
                    }
                }
            }
        }
    }
}
