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
    /// Digest the metadata entries in sorted key order without
    /// allocating a `Debug` string.
    pub fn digest_into(&self, hash: &mut crate::fnv::Fnv1a64) {
        let mut entries: Vec<(&str, &str)> = self
            .entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        entries.sort_unstable();
        hash.write_usize(entries.len());
        for (key, value) in entries {
            hash.write_bytes(key.as_bytes());
            hash.write_u8(0);
            hash.write_bytes(value.as_bytes());
            hash.write_u8(0);
        }
    }

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
