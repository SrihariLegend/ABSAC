use std::collections::HashMap;
use sir_types::{CostProfile, RegionId};

/// Database mapping regions to their pre-computed cost profiles.
///
/// Populated by `CostDeriver` during semantic derivation.
/// Parallel to `StructuralDatabase` — cost is not structure.
/// Immutable after `CostDeriver::derive()` completes.
#[derive(Clone, Debug, Default)]
pub struct CostDatabase {
    costs: HashMap<RegionId, CostProfile>,
}

impl CostDatabase {
    /// Create an empty cost database.
    pub fn new() -> Self {
        Self {
            costs: HashMap::new(),
        }
    }

    /// Store the cost profile for a region.
    pub fn insert(&mut self, region: RegionId, profile: CostProfile) {
        self.costs.insert(region, profile);
    }

    /// Retrieve the cost profile for a region, if present.
    pub fn for_region(&self, region: RegionId) -> Option<&CostProfile> {
        self.costs.get(&region)
    }

    /// Number of regions with cost data.
    pub fn len(&self) -> usize {
        self.costs.len()
    }

    /// Whether the database is empty.
    pub fn is_empty(&self) -> bool {
        self.costs.is_empty()
    }
}
