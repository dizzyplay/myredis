#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use crate::protocol::encoder::RedisEncoder;

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
        encoder.encode_bulk_string(&mut dst, "hello\r\nworld");
        assert_eq!(&dst[..], b"$12\r\nhello\r\nworld\r\n");
    }

    #[test]
    fn test_encode_array() {
        let encoder = RedisEncoder::new();
        let mut dst = BytesMut::new();

        // Test normal array
        let items = ["hello", "world"];
        encoder.encode_array(&mut dst, &items);
        assert_eq!(&dst[..], b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n");

        // Test empty array
        dst.clear();
        encoder.encode_empty_array(&mut dst);
        assert_eq!(&dst[..], b"*0\r\n");

        // Test null array
        dst.clear();
        encoder.encode_null_array(&mut dst);
        assert_eq!(&dst[..], b"*-1\r\n");

        // Test array with empty string
        dst.clear();
        let items = ["", "test"];
        encoder.encode_array(&mut dst, &items);
        assert_eq!(&dst[..], b"*2\r\n$0\r\n\r\n$4\r\ntest\r\n");

        // Test single item array (implicitly tests array header)
        dst.clear();
        let items = ["single"];
        encoder.encode_array(&mut dst, &items);
        assert_eq!(&dst[..], b"*1\r\n$6\r\nsingle\r\n");
    }
}