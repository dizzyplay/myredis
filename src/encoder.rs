use bytes::BytesMut;

/// Redis 프로토콜의 인코딩을 담당하는 구조체
pub struct RedisEncoder;

impl RedisEncoder {
    /// 새로운 Encoder 인스턴스를 생성합니다.
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
}
