use std::collections::HashMap;

use sir_transform::constraints::Constraint;
use sir_transform::structures::SourceStructure;

use crate::region::RegionId;

/// Describes the physical organization of data in a region.
/// Entirely deterministic — derived from SIR types and analysis facts.
#[derive(Clone, Debug)]
pub struct StructuralDescription {
    pub region: RegionId,
    pub source_structure: SourceStructure,
    pub constraints: std::collections::HashSet<Constraint>,
}

impl StructuralDescription {
    pub fn new(
        region: RegionId,
        source_structure: SourceStructure,
    ) -> Self {
        Self {
            region,
            source_structure,
            constraints: std::collections::HashSet::new(),
        }
    }

    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.insert(constraint);
        self
    }
}

/// The structural knowledge database.
/// Stores deterministic descriptions of data organization per region.
#[derive(Clone, Debug, Default)]
pub struct StructuralDatabase {
    descriptions: HashMap<RegionId, StructuralDescription>,
    next_region_id: u64,
}

impl StructuralDatabase {
    pub fn new() -> Self {
        Self { descriptions: HashMap::new(), next_region_id: 0 }
    }

    pub fn add_description(&mut self, desc: StructuralDescription) {
        self.descriptions.insert(desc.region, desc);
    }

    pub fn region(&self, id: RegionId) -> Option<&StructuralDescription> {
        self.descriptions.get(&id)
    }

    pub fn regions(&self) -> impl Iterator<Item = (RegionId, &StructuralDescription)> {
        self.descriptions.iter().map(|(&id, desc)| (id, desc))
    }

    pub fn region_count(&self) -> usize {
        self.descriptions.len()
    }

    pub(crate) fn next_region_id(&mut self) -> RegionId {
        let id = RegionId::new(self.next_region_id);
        self.next_region_id += 1;
        id
    }
}
