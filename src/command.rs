use std::collections::VecDeque;
use std::sync::Arc;
use bytes::Bytes;
use tokio::io::AsyncWriteExt;
use crate::store::Store;
use anyhow::Result;

pub async fn process_command(command_list: &mut VecDeque<String>, store: &Arc<Store>) -> Result<Bytes> {
    if let Some(command) = command_list.pop_front() {
        match command.as_str() {
            "ECHO" => {
                let s = command_list.pop_front().unwrap();
                let formatted = format!("${}\r\n{}\r\n", s.len(), s);
                return Ok(Bytes::from(formatted))
            }
            "SET" => {
                let key = command_list.pop_front().unwrap();
                let value = command_list.pop_front().unwrap();
                store.insert(key, value).await;
                let formatted = format!("${}\r\n{}\r\n", "OK".len(), "OK");
                return Ok(Bytes::from(formatted))
            }
            "GET" => {
                let key = command_list.pop_front().unwrap();
                if let Some(value) = store.get(&key).await {
                    let formatted = format!("${}\r\n{}\r\n", value.len(), value);
                    return Ok(Bytes::from(formatted))
                } else {
                    return Ok(Bytes::from("$-1\r\n"))
                }
            }
            _ => {
                return Ok(Bytes::from("+PONG\r\n"))
            }
        }
    }
        
    Err(anyhow::anyhow!("Command not found or incomplete"))
}