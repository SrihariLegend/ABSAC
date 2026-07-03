use std::collections::{BTreeSet, HashMap};
use sir_types::NodeId;

use crate::concepts::SemanticConcept;

/// A region identifier — unique within a semantic database.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

/// Why a concept was recognized — deterministic, not heuristic.
#[derive(Clone, Debug)]
pub struct RecognitionExplanation {
    pub concept: SemanticConcept,
    pub triggering_facts: Vec<&'static str>,
}

/// A contiguous subgraph representing a semantic unit.
///
/// For v0.1, a region is simply a set of nodes involved in a
/// recognized computation (e.g., a loop body and its enclosing
/// array access). Region identification is intentionally minimal
/// and will become more sophisticated in future phases.
#[derive(Clone, Debug)]
pub struct Region {
    pub id: RegionId,
    pub nodes: BTreeSet<NodeId>,
    concepts: std::collections::HashSet<SemanticConcept>,
    explanations: HashMap<SemanticConcept, RecognitionExplanation>,
}

impl Region {
    pub fn new(id: RegionId) -> Self {
        Self {
            id,
            nodes: BTreeSet::new(),
            concepts: std::collections::HashSet::new(),
            explanations: HashMap::new(),
        }
    }

    /// Attach a concept to this region with an explanation.
    pub fn add_concept(&mut self, concept: SemanticConcept, explanation: RecognitionExplanation) {
        self.concepts.insert(concept);
        self.explanations.insert(concept, explanation);
    }

    /// Check whether this region carries a specific concept.
    pub fn contains(&self, concept: SemanticConcept) -> bool {
        self.concepts.contains(&concept)
    }

    /// All concepts attached to this region.
    pub fn concepts(&self) -> &std::collections::HashSet<SemanticConcept> {
        &self.concepts
    }

    /// The SIR nodes that constitute this region.
    pub fn nodes(&self) -> &BTreeSet<NodeId> {
        &self.nodes
    }

    /// Get the recognition explanation for a concept, if present.
    pub fn explanation(&self, concept: SemanticConcept) -> Option<&RecognitionExplanation> {
        self.explanations.get(&concept)
    }
}
