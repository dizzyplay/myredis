// store.rs
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Store {
	data: Mutex<HashMap<String, String>>,
}

impl Store {
	pub fn new() -> Self {
		Store {
			data: Mutex::new(HashMap::new())
		}
	}

	pub async fn insert(&self, key: String, value: String) {
		let mut a = self.data.lock().await;
		a.insert(key, value);
	}

	pub async fn get(&self, key: &str) -> Option<String> {
		let a = self.data.lock().await;
		a.get(key).cloned()
	}
}

// Wrap the Store in a Mutex
