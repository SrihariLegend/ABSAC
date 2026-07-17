use std::collections::HashMap;

use sir_transform::constraints::Constraint;
use sir_transform::roles::RegionRoles;
use sir_transform::structures::SourceStructure;

use crate::region::RegionId;

/// Describes the physical organization of data in a region.
/// Entirely deterministic — derived from SIR types and analysis facts.
#[derive(Clone, Debug)]
pub struct StructuralDescription {
    pub region: RegionId,
    pub source_structure: SourceStructure,
    pub roles: Vec<RegionRoles>,
    pub constraints: std::collections::HashSet<Constraint>,
}

impl StructuralDescription {
    pub fn new(region: RegionId, source_structure: SourceStructure) -> Self {
        Self {
            region,
            source_structure,
            roles: Vec::new(),
            constraints: std::collections::HashSet::new(),
        }
    }

    pub fn with_roles(mut self, roles: RegionRoles) -> Self {
        self.roles.push(roles);
        self
    }
    
    pub fn add_role(&mut self, role: RegionRoles) {
        self.roles.push(role);
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
}

impl StructuralDatabase {
    pub fn new() -> Self {
        Self {
            descriptions: HashMap::new(),
        }
    }

    pub fn add_description(&mut self, desc: StructuralDescription) {
        debug_assert!(
            !self.descriptions.contains_key(&desc.region),
            "duplicate region {:?}",
            desc.region
        );
        self.descriptions.insert(desc.region, desc);
    }

    pub fn region(&self, id: RegionId) -> Option<&StructuralDescription> {
        self.descriptions.get(&id)
    }

    /// Get a mutable reference to a structural description.
    pub fn region_mut(&mut self, id: RegionId) -> Option<&mut StructuralDescription> {
        self.descriptions.get_mut(&id)
    }

    pub fn regions(&self) -> impl Iterator<Item = (RegionId, &StructuralDescription)> {
        self.descriptions.iter().map(|(&id, desc)| (id, desc))
    }

    pub fn region_count(&self) -> usize {
        self.descriptions.len()
    }

    /// Re-key a structural description from one region ID to another.
    ///
    /// This is needed because semantic region IDs may change after merging,
    /// while structural recognizers use placeholder region IDs.
    pub fn rekey_region(&mut self, from: RegionId, to: RegionId) {
        if let Some(mut desc) = self.descriptions.remove(&from) {
            desc.region = to;
            self.descriptions.insert(to, desc);
        }
    }
}
