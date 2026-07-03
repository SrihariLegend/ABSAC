use serde::{Deserialize, Serialize};

/// Properties that must be proven before transformation.
///
/// An Assumption is NOT yet established. It must eventually become
/// either Proven (by SMT or formal reasoning) or Refuted.
/// Assumptions must never be left unresolved.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Assumption {
    /// The transformed computation produces identical cardinality.
    EquivalentCardinality,
    /// The order of iteration is preserved (or does not matter).
    PreservesIterationOrder,
    /// The external memory layout is unchanged.
    PreservesLayout,
}
