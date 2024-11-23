// store.rs
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

#[derive(Debug)]
struct Value {
    data: String,
    expiry: Option<u64>, // 만료 시간 (Unix timestamp in milliseconds)
}

#[derive(Debug)]
pub struct Store {
    data: Mutex<HashMap<String, Value>>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            data: Mutex::new(HashMap::new())
        }
    }

    pub async fn insert(&self, key: String, value: String, expiry: Option<u64>) {
        let mut store = self.data.lock().await;
        let expiry_ts = expiry.map(|ms| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64 + ms
        });
        
        store.insert(key, Value {
            data: value,
            expiry: expiry_ts,
        });
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let mut store = self.data.lock().await;
        
        if let Some(value) = store.get(key) {
            // 만료 시간 체크
            if let Some(expiry) = value.expiry {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                
                if now > expiry {
                    // 만료된 키 삭제
                    store.remove(key);
                    return None;
                }
            }
            Some(value.data.clone())
        } else {
            None
        }
    }

    // RDB 파일 생성을 위한 데이터 iterator
    pub async fn iter_for_rdb(&self) -> impl Iterator<Item = (String, String, Option<u64>)> + '_ {
        let store = self.data.lock().await;
        store
            .iter()
            .map(|(k, v)| (k.clone(), v.data.clone(), v.expiry))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

// Wrap the Store in a Mutex
