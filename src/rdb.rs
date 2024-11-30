use crate::store::Store;
use crc::{Crc, CRC_64_MS};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct RDB;

impl RDB {
    // Helper function to encode integers in length-encoded format
    pub fn length_encode_int(value: usize, buffer: &mut Vec<u8>) {
        if value <= 63 { // 6 bits (00xxxxxx)
            buffer.push(value as u8);
        } else if value <= 16383 { // 14 bits (01xxxxxx xxxxxxxx)
            let first_byte = ((value >> 8) & 0x3F) as u8 | 0x40;
            let second_byte = (value & 0xFF) as u8;
            buffer.push(first_byte);
            buffer.push(second_byte);
        } else { // 32 bits (10xxxxxx + 4 bytes)
            buffer.push(0x80); // 10000000
            buffer.extend_from_slice(&(value as u32).to_be_bytes());
        }
    }

    pub fn length_decode_int(pos: &mut usize, buffer: &Vec<u8>) -> usize {
        match buffer[*pos] >> 6 {
            0 => {
                // next 6 bits is string length
                let num = (buffer[*pos] & 0b001111) as usize;
                *pos += 1;
                return num;
            }
            1 => {
                // read one additional byte. combined 14bits is string length
                let num = ((buffer[*pos] as usize) << 8 | buffer[*pos + 1] as usize) & 0x3FFF;
                *pos += 2;
                return num;
            }
            2 => {
                // discard current 6bits. The next 4 bytes the stream represent the length
                *pos += 5;
            }
            // 3
            _ => {
                // next 6bits
                let num = (buffer[*pos] & 0b001111) as usize;
                match num {
                    0 => {
                        // 0 is next 8bit integer
                        *pos += 1
                    }
                    1 => {
                        // 1 is next 16bit integer
                        *pos += 2
                    }
                    _ => {
                        // 2 is next 32bit integer
                        *pos += 3
                    }
                }
            }
        }
        panic!("FA decode fail");
    }

    pub async fn create_rdb<P: AsRef<Path> + std::fmt::Debug>(
        path: P,
        stores: Option<&[&Store]>,
    ) -> io::Result<()> {
        let mut buffer = Vec::new();
        println!("{:?}", path);
        // Redis RDB 파일의 매직 넘버와 버전을 작성
        buffer.extend_from_slice(b"REDIS0011");

        // redis-ver 메타데이터
        buffer.push(0xFA); // Auxiliary field marker
        Self::length_encode_int("redis-ver".len(), &mut buffer);
        buffer.extend_from_slice(b"redis-ver");
        Self::length_encode_int("7.2.0".len(), &mut buffer);
        buffer.extend_from_slice(b"7.2.0");

        // redis-bits 메타데이터
        buffer.push(0xFA); // Auxiliary field marker
        Self::length_encode_int("redis-bits".len(), &mut buffer);
        buffer.extend_from_slice(b"redis-bits");
        buffer.push(0xC0); // 특수 인코딩 표시 (11000000)
        buffer.push(0x40); // 64 비트 값

        // stores가 있는 경우에만 데이터 처리
        if let Some(stores) = stores {
            // 각 Store에 대해 처리
            for (db_index, store) in stores.iter().enumerate() {
                // 데이터베이스 선택
                buffer.push(0xFE); // Select DB
                buffer.push(db_index as u8);

                // Resizedb 필드
                buffer.push(0xFB); // Resizedb marker
                let hash_table_size = store.len().await;
                buffer.push(hash_table_size as u8); // Hash table size
                let expire_table_size = store.expire_len().await;
                buffer.push(expire_table_size as u8); // Expire hash table size

                // 데이터베이스 내용을 RDB 파일에 기록
                for (key, value, expiry) in store.iter_for_rdb().await {
                    match expiry {
                        Some(expiry_ts) => {
                            if expiry_ts <= SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64
                            {
                                continue;
                            }
                            buffer.push(0xFC); // 밀리초 단위 만료 시간
                            buffer.extend_from_slice(&expiry_ts.to_le_bytes());
                        }
                        None => {}
                    }

                    // 문자열 값 타입 마커
                    buffer.push(0x00);

                    // 키 길이와 키 데이터
                    buffer.push(key.len() as u8);
                    buffer.extend_from_slice(key.as_bytes());

                    // 값 길이와 값 데이터
                    buffer.push(value.len() as u8);
                    buffer.extend_from_slice(value.as_bytes());
                }
            }
        }

        // RDB 파일 끝 마커
        buffer.push(0xFF);

        // CRC64 체크섬 계산 및 추가 (big-endian 형식)
        let crc = Crc::<u64>::new(&CRC_64_MS);
        let checksum = crc.checksum(&buffer);
        buffer.extend_from_slice(&checksum.to_be_bytes());

        // 파일에 버퍼 내용 쓰기
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
                    // Auxiliary field
                    pos += 1;
                    // key length와 key
                    let key_len = Self::length_decode_int(&mut pos, &buffer);
                    pos += key_len; // key 건너뛰기
                    
                    // value length와 value
                    if pos < buffer.len() && buffer[pos] == 0xC0 { // 특수 인코딩
                        pos += 2; // 0xC0와 값 건너뛰기
                    } else {
                        let value_len = Self::length_decode_int(&mut pos, &buffer);
                        pos += value_len; // value 건너뛰기
                    }
                }
                0xFB => {
                    // Resizedb 필드
                    pos += 1;
                    // Hash table size
                    pos += 1;
                    // Expire hash table size
                    pos += 1;
                }
                0xFE => {
                    // 데이터베이스 선택자
                    pos += 2; // 선택자와 데이터베이스 인덱스 건너뛰기
                }
                0xFC => {
                    // 만료 시간이 있는 키-값 쌍
                    pos += 1;
                    let expiry = u64::from_le_bytes([
                        buffer[pos],
                        buffer[pos + 1],
                        buffer[pos + 2],
                        buffer[pos + 3],
                        buffer[pos + 4],
                        buffer[pos + 5],
                        buffer[pos + 6],
                        buffer[pos + 7],
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
                    let key = String::from_utf8_lossy(&buffer[pos..pos + key_len]).to_string();
                    pos += key_len;

                    // 값 읽기
                    let value_len = buffer[pos] as usize;
                    pos += 1;
                    let value = String::from_utf8_lossy(&buffer[pos..pos + value_len]).to_string();
                    pos += value_len;

                    store.insert(key, value, Some(expiry)).await;
                }
                0x00 => {
                    // 만료 시간이 없는 키-값 쌍
                    pos += 1;

                    // 키 읽기
                    let key_len = buffer[pos] as usize;
                    pos += 1;
                    let key = String::from_utf8_lossy(&buffer[pos..pos + key_len]).to_string();
                    pos += key_len;

                    // 값 읽기
                    let value_len = buffer[pos] as usize;
                    pos += 1;
                    let value = String::from_utf8_lossy(&buffer[pos..pos + value_len]).to_string();
                    pos += value_len;

                    store.insert(key, value, None).await;
                }
                0xFF => {
                    // EOF 마커
                    break;
                }
                _ => {
                    println!("here -> {:?} ", String::from_utf8_lossy(&buffer[pos..pos+1]));
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
