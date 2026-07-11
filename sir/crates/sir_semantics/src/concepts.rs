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
    /// Operation: checking if at least one element satisfies a condition
    DisjunctiveReduction,
    /// Operation: checking if all elements satisfy a condition
    ConjunctiveReduction,
    /// Operation: checking if an odd number of elements satisfy a condition (parity/xor)
    ExclusiveReduction,
    /// Operation: finding the first element that satisfies a condition
    FindFirst,
    /// Data: a stream of booleans generated dynamically by comparing elements
    PredicateCollection,
    /// Operation: checking if elements are present in two collections simultaneously
    SetIntersection,
    /// Operation: integer modulo by a power of two
    ModuloPowerOfTwo,
    /// Operation: integer multiplication by a power of two
    MultiplyPowerOfTwo,
    /// Operation: integer division by a power of two
    DividePowerOfTwo,
    /// Operation: shift left followed by shift right, extracting a mask
    ShiftMask,
}

impl fmt::Display for SemanticConcept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticConcept::BooleanCollection => write!(f, "BooleanCollection"),
            SemanticConcept::FiniteCollection => write!(f, "FiniteCollection"),
            SemanticConcept::MembershipTraversal => write!(f, "MembershipTraversal"),
            SemanticConcept::CardinalityReduction => write!(f, "CardinalityReduction"),
            SemanticConcept::DisjunctiveReduction => write!(f, "DisjunctiveReduction"),
            SemanticConcept::ConjunctiveReduction => write!(f, "ConjunctiveReduction"),
            SemanticConcept::ExclusiveReduction => write!(f, "ExclusiveReduction"),
            SemanticConcept::PredicateCollection => write!(f, "PredicateCollection"),
            SemanticConcept::FindFirst => write!(f, "FindFirst"),
            SemanticConcept::SetIntersection => write!(f, "SetIntersection"),
            SemanticConcept::ModuloPowerOfTwo => write!(f, "ModuloPowerOfTwo"),
            SemanticConcept::MultiplyPowerOfTwo => write!(f, "MultiplyPowerOfTwo"),
            SemanticConcept::DividePowerOfTwo => write!(f, "DividePowerOfTwo"),
            SemanticConcept::ShiftMask => write!(f, "ShiftMask"),
        }
    }
}
