use std::collections::BTreeMap;

use sir_generation::candidate::{Candidate, CandidateId};
use sir_types::RegionId;
use sir_verification::Proof;

use crate::score::{CostModelReport, TransformationScore};

/// Owned equivalent of SelectedCandidate for persistent storage.
///
/// NOTE: Serialize/Deserialize not yet derived — blocked on Candidate
/// (SemanticConcept, DefinitionId lack serde) and Proof (Theorem lacks serde).
#[derive(Clone, Debug)]
pub struct SelectedCandidateOwned {
    pub candidate: Candidate,
    pub proof: Proof,
    pub score: TransformationScore,
}

/// Owned equivalent of SelectionResult for persistent storage.
///
/// NOTE: Serialize/Deserialize not yet derived — see SelectedCandidateOwned.
#[derive(Clone, Debug)]
pub struct SelectionResultOwned {
    pub chosen: Option<SelectedCandidateOwned>,
    pub rejected: Vec<CandidateId>,
    pub report: CostModelReport,
}

pub struct SelectionDatabase {
    results: BTreeMap<RegionId, SelectionResultOwned>,
}

impl SelectionDatabase {
    pub fn new() -> Self {
        Self {
            results: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, region: RegionId, result: SelectionResultOwned) {
        self.results.insert(region, result);
    }

    pub fn get(&self, region: RegionId) -> Option<&SelectionResultOwned> {
        self.results.get(&region)
    }

    pub fn iter(&self) -> impl Iterator<Item = (RegionId, &SelectionResultOwned)> {
        self.results.iter().map(|(&k, v)| (k, v))
    }
}

impl Default for SelectionDatabase {
    fn default() -> Self {
        Self::new()
    }
}
