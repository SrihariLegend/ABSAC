use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::RegionId;

use crate::hypothesis::{EvidenceId, Representation};

/// Whether evidence supports or opposes a representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    Supports,
    Against,
}

/// Evidence is an observation — an instance about a specific region,
/// not a rule template. Each piece of evidence records which semantic
/// concept triggered it, which representation it affects, and how strongly.
#[derive(Clone, Debug)]
pub struct Evidence {
    pub region: RegionId,
    pub representation: Representation,
    pub polarity: Polarity,
    pub weight: u16,
    pub source: SemanticConcept,
    pub explanation: &'static str,
}

/// A flat registry of all evidence entries produced during inference.
///
/// Entries are reusable across regions — the same explanation applies
/// wherever the same concept triggers the same representation.
#[derive(Clone, Debug, Default)]
pub struct EvidenceRegistry {
    entries: Vec<Evidence>,
}

impl EvidenceRegistry {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Add an evidence entry and return its ID.
    pub fn add(&mut self, evidence: Evidence) -> EvidenceId {
        let id = self.entries.len();
        self.entries.push(evidence);
        id
    }

    /// Get an evidence entry by ID.
    pub fn get(&self, id: EvidenceId) -> Option<&Evidence> {
        self.entries.get(id)
    }

    /// All evidence entries.
    pub fn all(&self) -> &[Evidence] {
        &self.entries
    }

    /// Evidence entries relevant to a specific region.
    pub fn for_region(&self, region: RegionId) -> Vec<&Evidence> {
        self.entries.iter().filter(|e| e.region == region).collect()
    }
}
