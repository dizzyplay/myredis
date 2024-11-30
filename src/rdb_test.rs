use crate::rdb::RDB;
use crate::store::Store;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::test;

#[test]
async fn test_create_empty_rdb() {
    let path = "test_empty.rdb";
    
    // 빈 RDB 파일 생성
    RDB::create_rdb(path, None).await.unwrap();
    
    // 파일이 생성되었는지 확인
    assert!(Path::new(path).exists());
    
    // 파일 내용 확인
    let contents = fs::read(path).unwrap();
    
    // 매직 넘버와 버전 확인
    assert_eq!(&contents[0..9], b"REDIS0011");
    
    // 메타데이터 마커와 redis-ver 확인
    assert_eq!(contents[9], 0xFA);
    assert_eq!(contents[10], 0x09); // "redis-ver" 길이 (9) - length encoded
    assert_eq!(&contents[11..20], b"redis-ver");
    assert_eq!(contents[20], 0x05); // "7.2.0" 길이 (5) - length encoded
    assert_eq!(&contents[21..26], b"7.2.0");
    
    // redis-bits 메타데이터 확인
    assert_eq!(contents[26], 0xFA);
    assert_eq!(contents[27], 0x0A); // "redis-bits" 길이 (10) - length encoded
    assert_eq!(&contents[28..38], b"redis-bits");
    assert_eq!(contents[38], 0xC0); // 특수 인코딩
    assert_eq!(contents[39], 0x40); // 64비트
    
    // EOF 마커 확인
    assert_eq!(contents[40], 0xFF);
    
    // 체크섬이 8바이트인지 확인
    assert_eq!(contents.len(), 49); // 매직넘버(9) + 메타데이터(31) + EOF(1) + 체크섬(8)
    
    // 테스트 후 파일 삭제
    fs::remove_file(path).unwrap();
}

#[test]
async fn test_create_rdb_with_data() {
    let path = "test_data.rdb";
    
    // Store 생성 및 데이터 추가
    let store = Store::new();
    store.insert("key1".to_string(), "value1".to_string(), None).await;
    store.insert("key2".to_string(), "value2".to_string(), Some(60000)).await; // 60초 후 만료
    
    // RDB 파일 생성
    let stores = vec![&store];
    RDB::create_rdb(path, Some(&stores)).await.unwrap();
    
    // 파일이 생성되었는지 확인
    assert!(Path::new(path).exists());
    
    // 파일 내용 확인
    let contents = fs::read(path).unwrap();
    
    // 매직 넘버와 버전 확인
    assert_eq!(&contents[0..9], b"REDIS0011");
    
    // 데이터베이스 선택자(0xFE)가 있는지 확인
    assert!(contents.windows(2).any(|w| w[0] == 0xFE && w[1] == 0x00));
    
    // key1과 key2가 파일에 포함되어 있는지 확인
    let contents_str = String::from_utf8_lossy(&contents);
    assert!(contents_str.contains("key1"));
    assert!(contents_str.contains("value1"));
    assert!(contents_str.contains("key2"));
    assert!(contents_str.contains("value2"));
    
    // EOF 마커가 있는지 확인
    assert!(contents.windows(2).any(|w| w[0] == 0xFF));
    
    // 테스트 후 파일 삭제
    fs::remove_file(path).unwrap();
}

#[test]
async fn test_create_rdb_with_multiple_stores() {
    let path = "test_multiple.rdb";
    
    // 여러 Store 생성 및 데이터 추가
    let store0 = Store::new();
    let store1 = Store::new();
    
    store0.insert("key1".to_string(), "value1".to_string(), None).await;
    // 충분히 긴 만료 시간 사용 (10분)
    store1.insert("key2".to_string(), "value2".to_string(), Some(600_000)).await;
    
    // RDB 파일 생성
    let stores = vec![&store0, &store1];
    RDB::create_rdb(path, Some(&stores)).await.unwrap();
    
    // 파일이 생성되었는지 확인
    assert!(Path::new(path).exists());
    
    // 파일 내용 확인
    let contents = fs::read(path).unwrap();
    
    // 매직 넘버와 버전 확인
    assert_eq!(&contents[0..9], b"REDIS0011");
    
    // 두 개의 데이터베이스 선택자가 있는지 확인
    let db_selectors: Vec<_> = contents.windows(2)
        .filter(|w| w[0] == 0xFE && w[1] < 2)  // 데이터베이스 인덱스는 0과 1
        .collect();
    assert_eq!(db_selectors.len(), 2);
    
    // 모든 키와 값이 파일에 포함되어 있는지 확인
    let contents_str = String::from_utf8_lossy(&contents);
    assert!(contents_str.contains("key1"));
    assert!(contents_str.contains("value1"));
    assert!(contents_str.contains("key2"));
    assert!(contents_str.contains("value2"));
    
    // 만료 시간 마커와 값 확인 (store1의 key2)
    let expiry_marker_pos = contents.windows(2)
        .position(|w| w[0] == 0xFC)  // 밀리초 단위 만료 시간 마커
        .expect("만료 시간 마커를 찾을 수 없습니다");
    
    // 마커 다음에 오는 8바이트가 만료 시간을 나타내야 함
    let expiry_time_bytes = &contents[expiry_marker_pos + 1..expiry_marker_pos + 9];
    let expiry_time = u64::from_le_bytes(expiry_time_bytes.try_into().unwrap());
    
    // 현재 시간 가져오기
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    // 만료 시간이 현재 시간보다 크고, 현재 시간 + 10분 근처여야 함
    assert!(expiry_time > now);
    assert!(expiry_time <= now + 600_100);  // 100ms 정도의 여유를 둠
    
    // EOF 마커가 있는지 확인
    assert!(contents.windows(2).any(|w| w[0] == 0xFF));
    
    // 테스트 후 파일 삭제
    fs::remove_file(path).unwrap();
}

// 밀리초 단위 만료 시간 테스트 추가
#[test]
async fn test_create_rdb_with_millisecond_expiry() {
    let path = "test_ms_expiry.rdb";
    
    // Store 생성 및 데이터 추가 (밀리초 단위 만료 시간)
    let store = Store::new();
    store.insert("key1".to_string(), "value1".to_string(), Some(1500)).await;  // 1.5초 후 만료
    
    // RDB 파일 생성
    let stores = vec![&store];
    RDB::create_rdb(path, Some(&stores)).await.unwrap();
    
    // 파일 내용 확인
    let contents = fs::read(path).unwrap();
    
    // 매직 넘버와 버전 확인
    assert_eq!(&contents[0..9], b"REDIS0011");
    
    // 데이터베이스 선택자(0xFE)가 있는지 확인
    assert!(contents.windows(2).any(|w| w[0] == 0xFE && w[1] == 0x00));
    
    // 만료 시간 마커와 값 확인 (0xFC는 밀리초 단위 만료 시간을 나타냄)
    let expiry_marker_pos = contents.windows(2)
        .position(|w| w[0] == 0xFC)  // 밀리초 단위 만료 시간 마커
        .expect("만료 시간 마커를 찾을 수 없습니다");
    
    // 마커 다음에 오는 8바이트가 만료 시간을 나타내야 함
    let expiry_time_bytes = &contents[expiry_marker_pos + 1..expiry_marker_pos + 9];
    let expiry_time = u64::from_le_bytes(expiry_time_bytes.try_into().unwrap());
    
    // 현재 시간 가져오기
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    // 만료 시간이 현재 시간보다 크고, 현재 시간 + 1.5초 근처여야 함
    assert!(expiry_time > now);
    assert!(expiry_time <= now + 1600);  // 100ms 정도의 여유를 둠
    
    // EOF 마커가 있는지 확인
    assert!(contents.windows(2).any(|w| w[0] == 0xFF));
    
    // 테스트 후 파일 삭제
    fs::remove_file(path).unwrap();
}

#[test]
async fn test_length_encode_int() {
    let mut buffer = Vec::new();
    
    // Test case 1: 6비트 숫자 (63 이하)
    RDB::length_encode_int(63, &mut buffer);
    assert_eq!(buffer, vec![0x3F]);  // 00111111
    buffer.clear();
    
    // Test case 2: 14비트 숫자 (16383 이하)
    RDB::length_encode_int(16383, &mut buffer);
    assert_eq!(buffer, vec![0x7F, 0xFF]);  // 01111111 11111111
    buffer.clear();
    
    // Test case 3: 32비트 숫자
    RDB::length_encode_int(1000000, &mut buffer);
    assert_eq!(buffer, vec![0x80, 0x00, 0x0F, 0x42, 0x40]);  // 10000000 + 4바이트
    buffer.clear();
}

#[test]
async fn test_rdb_with_fb_field() {
    let path = "test_fb.rdb";
    
    // Create store with some data
    let store = Store::new();
    store.insert("key1".to_string(), "value1".to_string(), None).await;
    store.insert("key2".to_string(), "value2".to_string(), Some(60000)).await;
    
    // Create RDB file
    let stores = vec![&store];
    RDB::create_rdb(path, Some(&stores)).await.unwrap();
    
    // Read file contents
    let contents = fs::read(path).unwrap();
    
    // Find the database selector (0xFE) and check if FB field follows
    let mut found_fb = false;
    for i in 0..contents.len()-2 {
        if contents[i] == 0xFE && contents[i+1] == 0x00 {  // DB 0
            assert_eq!(contents[i+2], 0xFB);  // FB field
            found_fb = true;
            
            // Verify that two length-encoded integers follow
            // We expect small numbers, so they should be single bytes
            assert!(contents[i+3] < 0x80);  // Hash table size (MSB should be 0)
            assert!(contents[i+4] < 0x80);  // Expire hash table size (MSB should be 0)
            break;
        }
    }
    assert!(found_fb, "FB field not found after database selector");
    
    // Clean up
    fs::remove_file(path).unwrap();
}

#[test]
async fn test_read_rdb() {
    let path = "test_read.rdb";
    
    // 테스트용 Store 생성 및 데이터 추가
    let original_store = Store::new();
    original_store.insert("key1".to_string(), "value1".to_string(), None).await;
    
    // 현재 시간으로부터 60초 후 만료되는 키 추가
    let expiry = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64 + 60000;
    original_store.insert("key2".to_string(), "value2".to_string(), Some(expiry)).await;
    
    // RDB 파일 생성
    let stores = vec![&original_store];
    RDB::create_rdb(path, Some(&stores)).await.unwrap();
    
    // RDB 파일 읽기
    let loaded_store = RDB::read_rdb(path).await.unwrap();
    
    // 데이터 검증
    assert_eq!(loaded_store.get("key1").await.unwrap(), "value1");
    assert_eq!(loaded_store.get("key2").await.unwrap(), "value2");
    
    // 만료되지 않은 키는 여전히 존재해야 함
    assert!(loaded_store.get("key2").await.is_some());
    
    // 테스트 후 파일 삭제
    fs::remove_file(path).unwrap();
}

#[test]
async fn test_read_invalid_rdb() {
    let path = "test_invalid.rdb";
    
    // 잘못된 형식의 RDB 파일 생성
    fs::write(path, b"INVALID_RDB_FORMAT").unwrap();
    
    // 잘못된 형식의 파일 읽기 시도
    let result = RDB::read_rdb(path).await;
    assert!(result.is_err());
    
    // 테스트 후 파일 삭제
    fs::remove_file(path).unwrap();
}

#[test]
async fn test_read_empty_rdb() {
    let path = "test_empty_read.rdb";
    
    // 빈 RDB 파일 생성
    RDB::create_rdb(path, None).await.unwrap();
    
    // 빈 RDB 파일 읽기
    let store = RDB::read_rdb(path).await.unwrap();
    
    // 빈 store 확인
    assert!(store.get("non_existent_key").await.is_none());
    
    // 테스트 후 파일 삭제
    fs::remove_file(path).unwrap();
}

#[test]
async fn test_read_rdb_with_hex() {
    let path = "test_hex.rdb";
    
    // 정확한 헥스값으로 파일 생성
    let hex_str = "52 45 44 49 53 30 30 31 31 fa 09 72 65 64 69 73 2d 76 65 72 05 37 2e 32 2e 30 fa 0a 72 65 64 69 73 2d 62 69 74 73 c0 40 fe 00 fb 05 00 00 09 62 6c 75 65 62 65 72 72 79 05 67 72 61 70 65 00 0a 73 74 72 61 77 62 65 72 72 79 09 72 61 73 70 62 65 72 72 79 00 09 72 61 73 70 62 65 72 72 79 0a 73 74 72 61 77 62 65 72 72 79 00 05 67 72 61 70 65 06 6f 72 61 6e 67 65 00 06 62 61 6e 61 6e 61 05 61 70 70 6c 65 ff e4 a8 64 89 fc a8 37 0e 0a";
    
    // 헥스 문자열을 바이트 배열로 변환
    let hex_data: Vec<u8> = hex_str
        .split_whitespace()
        .map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect();
    
    // 파일에 헥스값 쓰기
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(&hex_data).unwrap();
    
    // RDB 파일 읽기
    let store = RDB::read_rdb(path).await.unwrap();
    
    // 저장된 키-값 쌍 확인
    let expected_pairs = [
        ("blueberry", "grape"),
        ("strawberry", "raspberry"),
        ("raspberry", "strawberry"),
        ("grape", "orange"),
        ("banana", "apple"),
    ];
    
    for (key, expected_value) in expected_pairs.iter() {
        let value = store.get(key).await.unwrap();
        assert_eq!(&value, expected_value);
    }
    
    // 총 키-값 쌍 개수 확인
    assert_eq!(store.len().await, 5);
    
    // 테스트 후 파일 삭제
    fs::remove_file(path).unwrap();
}
