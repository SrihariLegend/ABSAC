//! Candidate generation engine.
//!
//! Generates candidate transformation plans from transformation contexts.
//! Pure — no SIR access, no ranking, no verification.

use std::collections::HashMap;

use sir_semantics::region::RegionId;
use sir_transform::context::TransformationContextDatabase;

use crate::candidate::{Candidate, CandidateId};

/// Stores candidate plans per region.
#[derive(Clone, Debug, Default)]
pub struct CandidateDatabase {
    candidates: HashMap<RegionId, Vec<Candidate>>,
    next_candidate_id: u64,
}

impl CandidateDatabase {
    pub fn new() -> Self {
        Self { candidates: HashMap::new(), next_candidate_id: 0 }
    }

    pub fn add(&mut self, region: RegionId, candidate: Candidate) {
        self.candidates.entry(region).or_default().push(candidate);
    }

    pub fn candidates(&self, region: RegionId) -> &[Candidate] {
        self.candidates.get(&region).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn all_candidates(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates.values().flat_map(|v| v.iter())
    }

    pub fn region_count(&self) -> usize {
        self.candidates.len()
    }

    pub(crate) fn next_id(&mut self) -> CandidateId {
        let id = CandidateId::new(self.next_candidate_id);
        self.next_candidate_id += 1;
        id
    }

    /// Validate: no duplicate IDs, all candidates have non-empty effects.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen_ids = std::collections::HashSet::new();
        for candidate in self.all_candidates() {
            if !seen_ids.insert(candidate.id) {
                return Err(format!("Duplicate candidate ID: {}", candidate.id));
            }
            if candidate.effects.is_empty() {
                return Err(format!("Candidate {} has no effects", candidate.id));
            }
        }
        Ok(())
    }
}

/// Generates candidate transformation plans from contexts.
///
/// Pure — no SIR access, no ranking, no verification.
pub struct CandidateGenerator {
    db: CandidateDatabase,
}

impl CandidateGenerator {
    pub fn new() -> Self {
        Self { db: CandidateDatabase::new() }
    }

    pub fn database(&self) -> &CandidateDatabase {
        &self.db
    }

    /// Generate candidates for every transformation context.
    ///
    /// Each context is inspected by generator functions that produce
    /// candidates when applicable.
    pub fn generate(&mut self, context_db: &TransformationContextDatabase) {
        for (region_id, contexts) in context_db.contexts() {
            for ctx in contexts {
                let candidates = crate::generators::all_plans(ctx);
                for mut candidate in candidates {
                    candidate.id = self.db.next_id();
                    self.db.add(region_id, candidate);
                }
            }
        }
    }
}
