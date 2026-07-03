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

    /// Merge regions that share nodes into single regions.
    ///
    /// After all recognizers have run, related concepts (e.g., all concepts
    /// for the same loop + array computation) may end up in separate regions
    /// because each recognizer creates its own. This method finds overlapping
    /// regions (regions that share SIR node IDs) and merges them so that
    /// one computation maps to one region with all its semantic concepts.
    ///
    /// This is critical for evidence accumulation: a merged region with
    /// multiple concepts produces combined evidence weight, enabling
    /// strong support scores for the resulting representation hypothesis.
    pub(crate) fn merge_overlapping_regions(&mut self) {
        // Keep merging until no more overlaps exist
        loop {
            let ids: Vec<RegionId> = self.regions.keys().copied().collect();
            if ids.len() <= 1 {
                break;
            }

            let mut merged = false;

            'outer: for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let has_overlap = {
                        let ri = &self.regions[&ids[i]];
                        let rj = &self.regions[&ids[j]];
                        ri.nodes.intersection(&rj.nodes).next().is_some()
                    };

                    if has_overlap {
                        // Absorb region j into region i, then restart scanning
                        let rj = self.regions.remove(&ids[j]).unwrap();
                        if let Some(ri) = self.regions.get_mut(&ids[i]) {
                            for &nid in &rj.nodes {
                                ri.nodes.insert(nid);
                            }
                            let concepts: Vec<SemanticConcept> =
                                rj.concepts().iter().copied().collect();
                            for concept in concepts {
                                if let Some(expl) = rj.explanation(concept) {
                                    ri.add_concept(concept, expl.clone());
                                }
                            }
                        }
                        merged = true;
                        break 'outer;
                    }
                }
            }

            if !merged {
                break;
            }
        }

        // Update next_region_id to avoid reusing IDs
        let max_id = self
            .regions
            .keys()
            .map(|rid| rid.as_u64())
            .max()
            .unwrap_or(0);
        self.next_region_id = max_id + 1;
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
        use crate::recognizers::{
            boolean_collection, cardinality_reduction, finite_collection,
            membership_traversal,
        };

        let bc_recs = boolean_collection::recognize_boolean_collection(func, analysis);
        for (_concept, explanation, node_ids) in bc_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let finite_recs = finite_collection::recognize_finite_collection(func, analysis);
        for (_concept, explanation, node_ids) in finite_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let membership_recs =
            membership_traversal::recognize_membership_traversal(func, analysis);
        for (_concept, explanation, node_ids) in membership_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        let cardinality_recs =
            cardinality_reduction::recognize_cardinality_reduction(func, analysis);
        for (_concept, explanation, node_ids) in cardinality_recs {
            let rid = self.db.next_region_id();
            let mut region = Region::new(rid);
            for node_id in &node_ids {
                region.nodes.insert(*node_id);
            }
            region.add_concept(explanation.concept, explanation);
            self.db.add_region(region);
        }

        // Merge overlapping regions so that related concepts
        // (e.g., all concepts for the same loop/array computation)
        // end up in a single region. This enables combined evidence
        // accumulation in the inference engine.
        self.db.merge_overlapping_regions();
    }
}

impl Default for SemanticEngine {
    fn default() -> Self {
        Self::new()
    }
}
