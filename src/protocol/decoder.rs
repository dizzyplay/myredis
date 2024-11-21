use bytes::{BytesMut, Buf};

#[derive(Debug)]
pub enum RedisCommand {
    Ping,
    Set(String, String, Option<u64>), // key, value, expiry in milliseconds
    Get(String),
    Echo(String),
    ConfigGet(String),  
    Unknown,
}

#[derive(Clone)]
pub struct RedisDecoder;

impl RedisDecoder {
    pub fn new() -> Self {
        RedisDecoder
    }

    fn read_resp_array_length(&self, src: &mut BytesMut) -> Option<usize> {
        let mut length_str = Vec::new();
        while !src.is_empty() && src[0] != b'\r' {
            length_str.push(src[0]);
            src.advance(1);
        }
        
        // Skip \r\n
        if src.len() >= 2 {
            src.advance(2);
            std::str::from_utf8(&length_str)
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
        } else {
            None
        }
    }

    fn read_bulk_string(&self, src: &mut BytesMut) -> Option<String> {
        if src.is_empty() || src[0] != b'$' {
            return None;
        }
        src.advance(1);

        let length = self.read_resp_array_length(src)?;
        if src.len() < length + 2 {  // +2 for \r\n
            return None;
        }

        let string = String::from_utf8_lossy(&src[..length]).to_string();
        src.advance(length + 2); // Skip string content and \r\n
        Some(string)
    }

    pub fn decode(&self, src: &mut BytesMut) -> Option<RedisCommand> {
        if src.is_empty() {
            return None;
        }

        // RESP 프로토콜에서 배열은 *로 시작
        if src[0] == b'*' {
            src.advance(1);
            let length = self.read_resp_array_length(src)?;
            
            if length == 1 && src.len() >= 4 {
                if let Some(cmd) = self.read_bulk_string(src) {
                    if cmd == "PING" {
                        return Some(RedisCommand::Ping);
                    }
                }
            } else if length >= 3 {
                if let Some(cmd) = self.read_bulk_string(src) {
                    if cmd == "SET" {
                        let key = self.read_bulk_string(src)?;
                        let value = self.read_bulk_string(src)?;
                        
                        // PX 옵션 처리
                        let mut expiry = None;
                        if length == 5 {
                            if let Some(opt) = self.read_bulk_string(src) {
                                if opt.to_uppercase() == "PX" {
                                    if let Some(ms_str) = self.read_bulk_string(src) {
                                        if let Ok(ms) = ms_str.parse::<u64>() {
                                            expiry = Some(ms);
                                        }
                                    }
                                }
                            }
                        }
                        return Some(RedisCommand::Set(key, value, expiry));
                    }else if cmd.to_uppercase() == "CONFIG" {
                        if let Some(subcommand) = self.read_bulk_string(src) {
                            if subcommand.to_uppercase() == "GET" {
                                let parameter = self.read_bulk_string(src)?;
                                return Some(RedisCommand::ConfigGet(parameter));
                            }
                        }
                    }
                }
            } else if length == 2 {
                // GET or ECHO command
                if let Some(cmd) = self.read_bulk_string(src) {
                    match cmd.as_str() {
                        "GET" => {
                            let key = self.read_bulk_string(src)?;
                            return Some(RedisCommand::Get(key));
                        }
                        "ECHO" => {
                            let message = self.read_bulk_string(src)?;
                            return Some(RedisCommand::Echo(message));
                        }
                        _ => {}
                    }
                }
            }
            return Some(RedisCommand::Unknown);
        }

        Some(RedisCommand::Unknown)
    }
}