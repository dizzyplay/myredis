use std::collections::HashMap;

pub trait Pattern {
    fn matches(&self, key: &str) -> bool;
}

#[derive(Debug)]
pub struct WildCardPattern(pub String);

impl Pattern for WildCardPattern {
    fn matches(&self, _: &str) -> bool {
        true
    }
}

pub trait HashMapPatternExt {
    fn contains_key_pattern<P: Pattern>(&self, pattern: P) -> bool;
}

impl<K: AsRef<str>, V> HashMapPatternExt for HashMap<K, V> {
    fn contains_key_pattern<P: Pattern>(&self, pattern: P) -> bool {
        self.keys().any(|k| pattern.matches(k.as_ref()))
    }
}