use serde::{Deserialize, Serialize};

/// A region identifier — unique within a semantic database.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RegionId(pub u64);

impl RegionId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for RegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "region#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_id_creation() {
        let id = RegionId::new(42);
        assert_eq!(id.as_u64(), 42);
        assert_eq!(format!("{id}"), "region#42");
    }

    #[test]
    fn region_id_copy() {
        let a = RegionId::new(10);
        let b = a; // Copy
        assert_eq!(a, b);
    }
}
