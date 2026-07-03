use std::collections::HashMap;

use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;

use crate::concepts::SemanticConcept;
use crate::region::{Region, RegionId, RecognitionExplanation};

/// The semantic knowledge database.
///
/// Stores regions and their recognized concepts. Immutable after
/// the `SemanticEngine::derive()` call completes.
#[derive(Clone, Debug, Default)]
pub struct SemanticDatabase {
    regions: HashMap<RegionId, Region>,
    next_region_id: u64,
}

impl SemanticDatabase {
    /// Create an empty semantic database.
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            next_region_id: 0,
        }
    }

    /// Add a region to the database.
    pub fn add_region(&mut self, region: Region) {
        self.regions.insert(region.id, region);
    }

    /// Iterate over all regions.
    pub fn regions(&self) -> impl Iterator<Item = (RegionId, &Region)> {
        self.regions.iter().map(|(&id, region)| (id, region))
    }

    /// Get a specific region by ID.
    pub fn region(&self, id: RegionId) -> Option<&Region> {
        self.regions.get(&id)
    }

    /// Get the explanation for why a concept was recognized in a region.
    pub fn explain(
        &self,
        region: RegionId,
        concept: SemanticConcept,
    ) -> Option<&RecognitionExplanation> {
        self.regions
            .get(&region)
            .and_then(|r| r.explanation(concept))
    }

    /// Number of regions in the database.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Allocate the next region ID.
    pub(crate) fn next_region_id(&mut self) -> RegionId {
        let id = RegionId::new(self.next_region_id);
        self.next_region_id += 1;
        id
    }
}

/// The semantic derivation engine.
///
/// Transforms compiler facts into semantic truths by running
/// deterministic recognizers over the function graph.
pub struct SemanticEngine {
    db: SemanticDatabase,
}

impl SemanticEngine {
    /// Create a new semantic engine with an empty database.
    pub fn new() -> Self {
        Self {
            db: SemanticDatabase::new(),
        }
    }

    /// Access the semantic database (read-only after derivation).
    pub fn database(&self) -> &SemanticDatabase {
        &self.db
    }

    /// Derive semantic truths from the function graph and compiler facts.
    ///
    /// This calls each recognizer, which inspects the function's graph
    /// structure (for node kinds, types, and connectivity) and the
    /// analysis fact database (for trip counts, purity, escape, etc.).
    ///
    /// Recognized concepts are grouped into regions and stored in the
    /// `SemanticDatabase`.
    pub fn derive(&mut self, func: &Function, analysis: &FactDatabase) {
        // Recognizers are called in Tasks 4a-4d.
        // For now, this is a no-op -- the engine compiles but derives nothing.
        let _ = func;
        let _ = analysis;
    }
}

impl Default for SemanticEngine {
    fn default() -> Self {
        Self::new()
    }
}
