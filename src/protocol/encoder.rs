use bytes::BytesMut;

/// Redis 프로토콜의 인코딩을 담당하는 구조체
pub struct RedisEncoder;

impl RedisEncoder {
    pub fn new() -> RedisEncoder {
        RedisEncoder
    }

    pub fn encode_pong(&self, dst: &mut BytesMut) {
        dst.extend_from_slice(b"+PONG\r\n");
    }

    pub fn encode_ok(&self, dst: &mut BytesMut) {
        dst.extend_from_slice(b"+OK\r\n");
    }

    pub fn encode_error(&self, dst: &mut BytesMut) {
        dst.extend_from_slice(b"-ERR unknown command\r\n");
    }

    pub fn encode_null(&self, dst: &mut BytesMut) {
        dst.extend_from_slice(b"$-1\r\n");
    }

    pub fn encode_bulk_string(&self, dst: &mut BytesMut, s: &str) {
        dst.extend_from_slice(format!("${}\r\n{}\r\n", s.len(), s).as_bytes());
    }

    pub fn encode_array(&self, dst: &mut BytesMut, items: &[&str]) {
        dst.extend_from_slice(format!("*{}\r\n", items.len()).as_bytes());
        for item in items {
            self.encode_bulk_string(dst, item);
        }
    }

    /// 빈 배열
    pub fn encode_empty_array(&self, dst: &mut BytesMut) {
        dst.extend_from_slice(b"*0\r\n");
    }

    /// null 배열
    pub fn encode_null_array(&self, dst: &mut BytesMut) {
        dst.extend_from_slice(b"*-1\r\n");
    }
}
