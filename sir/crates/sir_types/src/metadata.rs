use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Arbitrary key-value metadata attached to a node.
///
/// Metadata is used for debug information, optimization hints,
/// source-language annotations, and other extensible data that
/// doesn't fit into the fixed `Node` fields.
///
/// Both keys and values are `String`s for maximum flexibility and
/// JSON-serializability.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    entries: HashMap<String, String>,
}

impl Metadata {
    /// Create empty metadata.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Insert a key-value pair. Returns the old value if the key was present.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.entries.insert(key.into(), value.into())
    }

    /// Get the value for a key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }

    /// Remove a key and return its value, if present.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.entries.remove(key)
    }

    /// Return true if the metadata contains the given key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Return the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return an iterator over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut m = Metadata::new();
        assert!(m.is_empty());

        m.insert("source_language", "rust");
        assert_eq!(m.get("source_language"), Some("rust"));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn insert_override() {
        let mut m = Metadata::new();
        m.insert("hint", "inline");
        let old = m.insert("hint", "noinline");
        assert_eq!(old, Some("inline".to_string()));
        assert_eq!(m.get("hint"), Some("noinline"));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn remove() {
        let mut m = Metadata::new();
        m.insert("key", "value");
        assert_eq!(m.remove("key"), Some("value".to_string()));
        assert!(m.is_empty());
        assert_eq!(m.remove("nonexistent"), None);
    }

    #[test]
    fn contains_key() {
        let mut m = Metadata::new();
        m.insert("debug_name", "loop_counter");
        assert!(m.contains_key("debug_name"));
        assert!(!m.contains_key("missing"));
    }

    #[test]
    fn default_is_empty() {
        let m = Metadata::default();
        assert!(m.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let mut m = Metadata::new();
        m.insert("source_language", "rust");
        m.insert("original_syntax", "if x > 0 { a } else { b }");
        let json = serde_json::to_string(&m).unwrap();
        let parsed: Metadata = serde_json::from_str(&json).unwrap();
        assert_eq!(m, parsed);
    }
}
