#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use crate::protocol::decoder::{RedisDecoder, RedisCommand};

    fn create_buffer(data: &[u8]) -> BytesMut {
        let mut buffer = BytesMut::new();
        buffer.extend_from_slice(data);
        buffer
    }

    #[test]
    fn test_decode_ping() {
        let decoder = RedisDecoder::new();
        let mut buffer = create_buffer(b"*1\r\n$4\r\nPING\r\n");
        
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::Ping) => (),
            _ => panic!("Expected PING command"),
        }
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_decode_set() {
        let decoder = RedisDecoder::new();
        let mut buffer = create_buffer(b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n");
        
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::Set(key, value,None)) => {
                assert_eq!(key, "key");
                assert_eq!(value, "value");
            }
            _ => panic!("Expected SET command"),
        }
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_decode_set_long_key() {
        let decoder = RedisDecoder::new();
        let mut buffer = create_buffer(b"*3\r\n$3\r\nSET\r\n$10\r\nlongkeyaaa\r\n$5\r\nvalue\r\n");
        
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::Set(key, value, None)) => {
                assert_eq!(key, "longkeyaaa");
                assert_eq!(value, "value");
            }
            _ => panic!("Expected SET command"),
        }
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_decode_get() {
        let decoder = RedisDecoder::new();
        let mut buffer = create_buffer(b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
        
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::Get(key)) => {
                assert_eq!(key, "key");
            }
            _ => panic!("Expected GET command"),
        }
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_decode_unknown() {
        let decoder = RedisDecoder::new();
        let mut buffer = create_buffer(b"*2\r\n$3\r\nFOO\r\n$3\r\nkey\r\n");
        
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::Unknown) => (),
            _ => panic!("Expected Unknown command"),
        }
    }

    #[test]
    fn test_decode_empty_buffer() {
        let decoder = RedisDecoder::new();
        let mut buffer = BytesMut::new();
        
        assert!(decoder.decode(&mut buffer).is_none());
    }

    #[test]
    fn test_decode_incomplete_command() {
        let decoder = RedisDecoder::new();
        let mut buffer = create_buffer(b"*2\r\n$3\r\nGET\r\n");
        
        assert!(decoder.decode(&mut buffer).is_none());
    }

    #[test]
    fn test_decode_malformed_command() {
        let decoder = RedisDecoder::new();
        let mut buffer = create_buffer(b"GET key\r\n");
        
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::Unknown) => (),
            _ => panic!("Expected Unknown command for malformed input"),
        }
    }

    #[test]
    fn test_decode_echo() {
        let decoder = RedisDecoder::new();
        let mut buffer = create_buffer(b"*2\r\n$4\r\nECHO\r\n$5\r\nhello\r\n");
        
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::Echo(message)) => {
                assert_eq!(message, "hello");
            }
            _ => panic!("Expected ECHO command"),
        }
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_decode_config_get() {
        let decoder = RedisDecoder::new();
        
        // Test CONFIG GET maxclients
        let mut buffer = create_buffer(b"*3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$10\r\nmaxclients\r\n");
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::ConfigGet(param)) => {
                assert_eq!(param, "maxclients");
            }
            _ => panic!("Expected CONFIG GET command")

        }
        assert_eq!(buffer.len(), 0);

        // Test CONFIG GET timeout
        let mut buffer = create_buffer(b"*3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$7\r\ntimeout\r\n");
        match decoder.decode(&mut buffer) {
            Some(RedisCommand::ConfigGet(param)) => {
                assert_eq!(param, "timeout");
            }
            _ => panic!("Expected CONFIG GET command"),
        }
        assert_eq!(buffer.len(), 0);
    }
}