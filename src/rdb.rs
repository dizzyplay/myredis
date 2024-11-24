use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::store::Store;
use crc::{Crc, CRC_64_MS};

#[derive(Debug)]
pub struct RDB;

impl RDB {
    pub async fn create_rdb<P: AsRef<Path> + std::fmt::Debug>(path: P, stores: Option<&[&Store]>) -> io::Result<()> {
        let mut buffer = Vec::new();
        println!("{:?}", path);
        // Redis RDB 파일의 매직 넘버와 버전을 작성
        buffer.extend_from_slice(b"REDIS0011");
        
        // 메타데이터 영역 작성
        buffer.push(0xFA);
        buffer.extend_from_slice(b"redis-ver6.0.16");
        
        // stores가 있는 경우에만 데이터 처리
        if let Some(stores) = stores {
            // 각 Store에 대해 처리
            for (db_index, store) in stores.iter().enumerate() {
                let mut has_data = false;
                let mut db_buffer = Vec::new();
                
                // 데이터베이스 내용을 RDB 파일에 기록
                for (key, value, expiry) in store.iter_for_rdb().await {
                    match expiry {
                        // 만료 시간이 있는 경우
                        Some(expiry_ts) => {
                            // 이미 만료된 경우 저장하지 않음
                            if expiry_ts <= SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64 {
                                continue;
                            }
                            
                            // 밀리초 단위의 절대적인 Unix 타임스탬프를 RDB 파일에 기록
                            db_buffer.push(0xFC);  // 밀리초 단위 만료 시간
                            db_buffer.extend_from_slice(&expiry_ts.to_le_bytes());
                        },
                        // 만료 시간이 없는 경우
                        None => {}
                    }
                    
                    // 값 타입 마커 (문자열)
                    db_buffer.push(0x00);
                    
                    // 키 길이와 키 데이터
                    db_buffer.push(key.len() as u8);
                    db_buffer.extend_from_slice(key.as_bytes());
                    
                    // 값 길이와 값 데이터
                    db_buffer.push(value.len() as u8);
                    db_buffer.extend_from_slice(value.as_bytes());
                    
                    has_data = true;
                }
                
                // 데이터가 있는 경우에만 데이터베이스 선택자와 내용을 추가
                if has_data {
                    buffer.push(0xFE);
                    buffer.push(db_index as u8);
                    buffer.extend_from_slice(&db_buffer);
                }
            }
        }
        
        // EOF 마커
        buffer.push(0xFF);
        
        // 체크섬 계산 (Redis ECMA 체크섬 사용)
        let crc64 = Crc::<u64>::new(&CRC_64_MS);
        let checksum = crc64.checksum(&buffer);
        
        // 체크섬을 버퍼에 추가
        buffer.extend_from_slice(&checksum.to_le_bytes());
        
        // 버퍼의 모든 내용을 파일에 한 번에 쓰기
        let mut file = File::create(path)?;
        file.write_all(&buffer)?;
        
        Ok(())
    }

    pub async fn read_rdb<P: AsRef<Path>>(path: P) -> io::Result<Store> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        io::Read::read_to_end(&mut file, &mut buffer)?;

        println!("RDB file contents:");
        for (i, &byte) in buffer.iter().enumerate() {
            print!("{:02X} ", byte);
            if (i + 1) % 16 == 0 {
                println!();
            }
        }
        println!();

        // Redis RDB 파일의 매직 넘버와 버전 확인 (REDIS0011)
        if buffer.len() < 9 || &buffer[0..9] != b"REDIS0011" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid RDB file format",
            ));
        }

        let store = Store::new();
        let mut pos = 9; // 매직 넘버와 버전 다음부터 시작

        while pos < buffer.len() {
            match buffer[pos] {
                0xFA => {
                    // 메타데이터 영역 건너뛰기
                    pos += 1;
                    while pos < buffer.len() {
                        let mut found_end = false;
                        let mut current_pos = pos;
                        
                        // 메타데이터의 키-값 쌍 읽기
                        while current_pos < buffer.len() {
                            let len = buffer[current_pos] as usize;
                            if len == 0 {
                                found_end = true;
                                break;
                            }
                            current_pos += len + 1;
                        }
                        
                        if found_end {
                            pos = current_pos + 1;
                            break;
                        }
                        pos += 1;
                    }
                }
                0xFE => {
                    // 데이터베이스 선택자
                    pos += 2; // 선택자와 데이터베이스 인덱스 건너뛰기
                }
                0xFC => {
                    // 만료 시간이 있는 키-값 쌍
                    pos += 1;
                    let expiry = u64::from_le_bytes([
                        buffer[pos], buffer[pos+1], buffer[pos+2], buffer[pos+3],
                        buffer[pos+4], buffer[pos+5], buffer[pos+6], buffer[pos+7],
                    ]);
                    pos += 8;

                    // 값 타입 마커 확인
                    if buffer[pos] != 0x00 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Unsupported value type",
                        ));
                    }
                    pos += 1;

                    // 키 읽기
                    let key_len = buffer[pos] as usize;
                    pos += 1;
                    let key = String::from_utf8_lossy(&buffer[pos..pos+key_len]).to_string();
                    pos += key_len;

                    // 값 읽기
                    let value_len = buffer[pos] as usize;
                    pos += 1;
                    let value = String::from_utf8_lossy(&buffer[pos..pos+value_len]).to_string();
                    pos += value_len;

                    store.insert(key, value, Some(expiry)).await;
                }
                0x00 => {
                    // 만료 시간이 없는 키-값 쌍
                    pos += 1;

                    // 키 읽기
                    let key_len = buffer[pos] as usize;
                    pos += 1;
                    let key = String::from_utf8_lossy(&buffer[pos..pos+key_len]).to_string();
                    pos += key_len;

                    // 값 읽기
                    let value_len = buffer[pos] as usize;
                    pos += 1;
                    let value = String::from_utf8_lossy(&buffer[pos..pos+value_len]).to_string();
                    pos += value_len;

                    store.insert(key, value, None).await;
                }
                0xFF => {
                    // 파일의 끝
                    break;
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid RDB file format",
                    ));
                }
            }
        }

        Ok(store)
    }
}