// store.rs
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct Store {
	data: HashMap<String, String>,
}

impl Store {
	pub fn new() -> Self {
		Store {
			data: HashMap::new(),
		}
	}

	pub fn insert(&mut self, key: String, value: String) {
		self.data.insert(key, value);
	}

	pub fn get(&self, key: &str) -> Option<String> {
		self.data.get(key).cloned()
	}
}

// Wrap the Store in a Mutex
pub type SafeStore = Mutex<Store>;

// Convenience function to create a new SafeStore
pub fn new_safe_store() -> SafeStore {
	Mutex::new(Store::new())
}