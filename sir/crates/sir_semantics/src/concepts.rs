use std::fmt;

/// A semantic concept describing what a computation is doing.
///
/// Concepts are organized into two groups:
/// - **Data concepts:** describe the data being operated on
/// - **Operation concepts:** describe what the computation does with the data
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticConcept {
    /// Data: collection of boolean values (e.g., `bool[64]`)
    BooleanCollection,
    /// Data: collection with a statically known bound
    FiniteCollection,
    /// Operation: iterating over elements and testing membership
    MembershipTraversal,
    /// Operation: counting how many elements satisfy a condition
    CardinalityReduction,
}

impl fmt::Display for SemanticConcept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticConcept::BooleanCollection => write!(f, "BooleanCollection"),
            SemanticConcept::FiniteCollection => write!(f, "FiniteCollection"),
            SemanticConcept::MembershipTraversal => write!(f, "MembershipTraversal"),
            SemanticConcept::CardinalityReduction => write!(f, "CardinalityReduction"),
        }
    }
}
