use crate::rdb::RDB;
use crate::store::Store;
use std::fs;
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
    
    // 메타데이터 마커와 버전 정보 확인
    assert_eq!(contents[9], 0xFA);
    assert!(contents[10..].starts_with(b"redis-ver"));
    
    // EOF 마커가 있는지 확인
    assert!(contents.windows(2).any(|w| w[0] == 0xFF));
    
    // 파일 크기가 최소 크기(매직넘버 + 메타데이터 + EOF + 체크섬) 이상인지 확인
    assert!(contents.len() > 9 + 12 + 1 + 8);
    
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
