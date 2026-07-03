//! ProofObligation and ProofObligationDatabase.

use std::collections::HashMap;

use sir_generation::candidate::CandidateId;
use sir_transform::assumptions::Assumption;
use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
use sir_types::RegionId;

use crate::semantic::theorem::Theorem;
use crate::semantic::value::{Environment, Value};

/// A self-contained verification problem.
///
/// No SIR references — portable across backends.
/// Everything needed to verify equivalence is encoded here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofObligation {
    pub id: ObligationId,
    pub region: RegionId,
    pub candidate: CandidateId,
    pub definition: DefinitionId,
    pub theorem: Theorem,
    pub assumptions: Vec<Assumption>,
    pub domain: Option<FiniteDomain>,
}

/// Describes the input space for exhaustive enumeration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteDomain {
    pub variables: Vec<VariableSpec>,
}

impl FiniteDomain {
    /// Compute total state count from variable specs.
    /// Returns None on overflow (e.g., bool[65] exceeds u64::MAX).
    /// The exhaustive verifier treats overflow as DomainTooLarge.
    pub fn total_states(&self) -> Option<u64> {
        let mut total: u64 = 1;
        for var in &self.variables {
            let states = var.state_count()?;
            total = total.checked_mul(states)?;
        }
        Some(total)
    }

    /// Enumerate all input combinations in deterministic order.
    pub fn enumerate(&self) -> DomainIterator {
        DomainIterator {
            domain: self.clone(),
            index: 0,
            total: self.total_states(),
        }
    }
}

/// Specification for a single variable in the domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableSpec {
    pub id: VariableId,
    pub kind: VariableKind,
}

impl VariableSpec {
    fn state_count(&self) -> Option<u64> {
        match &self.kind {
            VariableKind::BooleanArray { length } => {
                let len = *length as u32;
                if len >= 64 {
                    None // 2^64 or larger overflows u64
                } else {
                    Some(1u64 << len)
                }
            }
        }
    }
}

/// The kind of a domain variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableKind {
    /// A fixed-size array of booleans. Induces 2^length possible states.
    BooleanArray { length: usize },
}

/// Iterates over all input combinations in a deterministic order.
#[derive(Clone, Debug)]
pub struct DomainIterator {
    domain: FiniteDomain,
    index: u64,
    total: Option<u64>,
}

impl Iterator for DomainIterator {
    type Item = Environment;

    fn next(&mut self) -> Option<Self::Item> {
        match self.total {
            Some(t) if self.index >= t => return None,
            None => return None, // overflowed domain
            _ => {}
        }

        let env = self.build_environment(self.index);
        self.index += 1;
        Some(env)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.total {
            Some(t) => {
                let remaining = t.saturating_sub(self.index);
                let rem_usize = remaining.min(usize::MAX as u64) as usize;
                (rem_usize, Some(rem_usize))
            }
            None => (0, None),
        }
    }
}

impl DomainIterator {
    /// Build an environment for the given state index.
    /// Uses a mixed-radix encoding: each variable gets its slice of the index bits.
    fn build_environment(&self, index: u64) -> Environment {
        let mut env = Environment::new();
        let mut bit_offset: u32 = 0;

        for var in &self.domain.variables {
            match &var.kind {
                VariableKind::BooleanArray { length } => {
                    let len = *length;
                    let mut bits = Vec::with_capacity(len);
                    for i in 0..len {
                        let bit = (index >> (bit_offset + i as u32)) & 1;
                        bits.push(bit != 0);
                    }
                    env.bind(var.id, Value::BooleanArray(bits));
                    bit_offset += len as u32;
                }
            }
        }

        env
    }
}

/// Stores proof obligations with indexed lookup.
///
/// Follows the pattern established by CandidateDatabase, FactDatabase, etc.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProofObligationDatabase {
    obligations: Vec<ProofObligation>,
    by_region: HashMap<RegionId, Vec<usize>>,
    by_candidate: HashMap<CandidateId, usize>,
    next_id: u64,
}

impl ProofObligationDatabase {
    pub fn new() -> Self {
        Self {
            obligations: Vec::new(),
            by_region: HashMap::new(),
            by_candidate: HashMap::new(),
            next_id: 0,
        }
    }

    /// Insert a proof obligation. Assigns an ID.
    pub fn insert(&mut self, mut obligation: ProofObligation) -> ObligationId {
        let id = ObligationId::new(self.next_id);
        self.next_id += 1;
        obligation.id = id;

        let index = self.obligations.len();
        self.by_region
            .entry(obligation.region)
            .or_default()
            .push(index);
        self.by_candidate
            .insert(obligation.candidate, index);
        self.obligations.push(obligation);

        id
    }

    /// Look up an obligation by its ID.
    pub fn get(&self, id: ObligationId) -> Option<&ProofObligation> {
        self.obligations.iter().find(|o| o.id == id)
    }

    /// Get all obligations for a region.
    pub fn for_region(&self, region: RegionId) -> Vec<&ProofObligation> {
        self.by_region
            .get(&region)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&i| self.obligations.get(i))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the obligation for a candidate, if one exists.
    pub fn for_candidate(&self, candidate: CandidateId) -> Option<&ProofObligation> {
        self.by_candidate
            .get(&candidate)
            .and_then(|&i| self.obligations.get(i))
    }

    /// Iterate over all obligations.
    pub fn all(&self) -> impl Iterator<Item = &ProofObligation> {
        self.obligations.iter()
    }

    /// Number of stored obligations.
    pub fn len(&self) -> usize {
        self.obligations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::expression::SemanticExpression;
    use sir_transform::ids::VariableId;

    #[test]
    fn finite_domain_boolean_array_4_total_states() {
        let domain = FiniteDomain {
            variables: vec![VariableSpec {
                id: VariableId::new(0),
                kind: VariableKind::BooleanArray { length: 4 },
            }],
        };
        assert_eq!(domain.total_states(), Some(16));
    }

    #[test]
    fn finite_domain_boolean_array_64_total_states() {
        let domain = FiniteDomain {
            variables: vec![VariableSpec {
                id: VariableId::new(0),
                kind: VariableKind::BooleanArray { length: 64 },
            }],
        };
        // 2^64 overflows u64
        assert_eq!(domain.total_states(), None);
    }

    #[test]
    fn finite_domain_empty_total_states() {
        let domain = FiniteDomain {
            variables: vec![],
        };
        assert_eq!(domain.total_states(), Some(1)); // empty product = 1
    }

    #[test]
    fn domain_iterator_bool4_enumerates_16_states() {
        let domain = FiniteDomain {
            variables: vec![VariableSpec {
                id: VariableId::new(0),
                kind: VariableKind::BooleanArray { length: 4 },
            }],
        };
        let envs: Vec<Environment> = domain.enumerate().collect();
        assert_eq!(envs.len(), 16);
    }

    #[test]
    fn domain_iterator_first_state_all_false() {
        let domain = FiniteDomain {
            variables: vec![VariableSpec {
                id: VariableId::new(0),
                kind: VariableKind::BooleanArray { length: 4 },
            }],
        };
        let first = domain.enumerate().next().unwrap();
        let val = first.lookup(VariableId::new(0)).unwrap();
        assert_eq!(val, &Value::BooleanArray(vec![false, false, false, false]));
    }

    #[test]
    fn domain_iterator_last_state_all_true() {
        let domain = FiniteDomain {
            variables: vec![VariableSpec {
                id: VariableId::new(0),
                kind: VariableKind::BooleanArray { length: 4 },
            }],
        };
        let last = domain.enumerate().last().unwrap();
        let val = last.lookup(VariableId::new(0)).unwrap();
        assert_eq!(val, &Value::BooleanArray(vec![true, true, true, true]));
    }

    #[test]
    fn domain_iterator_is_deterministic() {
        let domain = FiniteDomain {
            variables: vec![VariableSpec {
                id: VariableId::new(0),
                kind: VariableKind::BooleanArray { length: 3 },
            }],
        };
        let run1: Vec<Environment> = domain.enumerate().collect();
        let run2: Vec<Environment> = domain.enumerate().collect();
        assert_eq!(run1.len(), run2.len());
        for (e1, e2) in run1.iter().zip(run2.iter()) {
            assert_eq!(
                e1.lookup(VariableId::new(0)),
                e2.lookup(VariableId::new(0))
            );
        }
    }

    #[test]
    fn proof_obligation_database_insert_and_lookup() {
        let theorem = Theorem::new(
            SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
            SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
        );
        let obl = ProofObligation {
            id: ObligationId::new(0), // will be overwritten by insert
            region: RegionId::new(1),
            candidate: CandidateId::new(42),
            definition: DefinitionId::new(0),
            theorem,
            assumptions: vec![],
            domain: None,
        };

        let mut db = ProofObligationDatabase::new();
        let assigned_id = db.insert(obl);

        let retrieved = db.get(assigned_id).unwrap();
        assert_eq!(retrieved.candidate, CandidateId::new(42));
        assert_eq!(retrieved.region, RegionId::new(1));
    }

    #[test]
    fn proof_obligation_database_for_candidate() {
        let theorem = Theorem::new(
            SemanticExpression::Constant(sir_types::ConstantData::u64(1)),
            SemanticExpression::Constant(sir_types::ConstantData::u64(1)),
        );
        let obl = ProofObligation {
            id: ObligationId::new(0),
            region: RegionId::new(0),
            candidate: CandidateId::new(7),
            definition: DefinitionId::new(0),
            theorem,
            assumptions: vec![],
            domain: None,
        };
        let mut db = ProofObligationDatabase::new();
        db.insert(obl);

        let found = db.for_candidate(CandidateId::new(7));
        assert!(found.is_some());
        assert!(db.for_candidate(CandidateId::new(999)).is_none());
    }
}
