use std::collections::HashMap;

use crate::RegionId;

/// A keyed collection of items per region. Common pattern across all databases.
#[derive(Clone, Debug)]
pub struct RegionMap<T> {
    entries: HashMap<RegionId, Vec<T>>,
}

impl<T> Default for RegionMap<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<T> RegionMap<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, region: RegionId, item: T) {
        self.entries.entry(region).or_default().push(item);
    }

    pub fn get(&self, region: RegionId) -> &[T] {
        self.entries
            .get(&region)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn iter(&self) -> impl Iterator<Item = (RegionId, &[T])> {
        self.entries
            .iter()
            .map(|(&rid, v)| (rid, v.as_slice()))
    }

    pub fn all(&self) -> impl Iterator<Item = &T> {
        self.entries.values().flat_map(|v| v.iter())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
