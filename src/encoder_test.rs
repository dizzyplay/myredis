#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use crate::encoder::RedisEncoder;

    #[test]
    fn test_encode_pong() {
        let encoder = RedisEncoder::new();
        let mut dst = BytesMut::new();
        encoder.encode_pong(&mut dst);
        assert_eq!(&dst[..], b"+PONG\r\n");
    }

    #[test]
    fn test_encode_ok() {
        let encoder = RedisEncoder::new();
        let mut dst = BytesMut::new();
        encoder.encode_ok(&mut dst);
        assert_eq!(&dst[..], b"+OK\r\n");
    }

    #[test]
    fn test_encode_error() {
        let encoder = RedisEncoder::new();
        let mut dst = BytesMut::new();
        encoder.encode_error(&mut dst);
        assert_eq!(&dst[..], b"-ERR unknown command\r\n");
    }

    #[test]
    fn test_encode_null() {
        let encoder = RedisEncoder::new();
        let mut dst = BytesMut::new();
        encoder.encode_null(&mut dst);
        assert_eq!(&dst[..], b"$-1\r\n");
    }

    #[test]
    fn test_encode_bulk_string() {
        let encoder = RedisEncoder::new();
        let mut dst = BytesMut::new();
        encoder.encode_bulk_string(&mut dst, "hello");
        assert_eq!(&dst[..], b"$5\r\nhello\r\n");

        // Test empty string
        dst.clear();
        encoder.encode_bulk_string(&mut dst, "");
        assert_eq!(&dst[..], b"$0\r\n\r\n");

        // Test string with special characters
        dst.clear();
        encoder.encode_bulk_string(&mut dst, "hello\nworld");
        assert_eq!(&dst[..], b"$11\r\nhello\nworld\r\n");
    }

    #[test]
    fn test_encode_echo() {
        let encoder = RedisEncoder::new();
        let mut dst = BytesMut::new();
        encoder.encode_bulk_string(&mut dst, "hello");
        assert_eq!(&dst[..], b"$5\r\nhello\r\n");
    }
}